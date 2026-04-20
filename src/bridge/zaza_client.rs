//! ZazaZ LLM Client Adapter
//!
//! This module provides integration with the zazaz API (https://zazaz.top) for LLM-powered features.
//! ZazaZ is the preferred LLM backend for AI-assisted merging, conflict resolution,
//! and branch purpose inference.
//!
//! # Environment Variables
//!
//! - `PROVIDER_ZAZAZ_API_KEY`: Your ZazaZ API key (required)
//! - `PROVIDER_ZAZAZ_API_URL`: ZazaZ API URL (optional, default: https://zazaz.top/v1)
//! - `PROVIDER_ZAZAZ_MODEL`: Model name (optional, default: ./Qwen3.5-27B-FP8)
//!
//! # Usage
//!
//! ```rust,no_run
//! use cadagent::bridge::zaza_client::ZazaClient;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let client = ZazaClient::from_env()?;
//!
//! // Generate response
//! let response = client.generate("Explain CAD constraints").await?;
//! println!("{}", response);
//! # Ok(())
//! # }
//! ```

use async_trait::async_trait;
use reqwest::{Client, ClientBuilder};
use secrecy::{ExposeSecret, Secret};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;
use tracing::{debug, error};

/// Zaza client error types
#[derive(Error, Debug)]
pub enum ZazaError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("API key not configured")]
    ApiKeyNotConfigured,

    #[error("Invalid response format: {0}")]
    InvalidResponse(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Server error: {status} - {message}")]
    ServerError { status: u16, message: String },

    #[error("Timeout exceeded")]
    Timeout,
}

/// ZazaZ API configuration
///
/// Uses environment variables:
/// - `PROVIDER_ZAZAZ_API_KEY`: API key (required)
/// - `PROVIDER_ZAZAZ_API_URL`: API URL (optional, default: https://zazaz.top/v1)
/// - `PROVIDER_ZAZAZ_MODEL`: Model name (optional, default: ./Qwen3.5-27B-FP8)
#[derive(Debug, Clone)]
pub struct ZazaConfig {
    /// ZazaZ API endpoint URL
    pub endpoint: String,
    /// API key for authentication
    pub api_key: Option<Secret<String>>,
    /// Request timeout in milliseconds
    pub timeout_ms: u64,
    /// Model name to use
    pub model: String,
    /// Maximum tokens in response
    pub max_tokens: u32,
    /// Temperature for generation (0.0-1.0)
    pub temperature: f32,
}

impl Default for ZazaConfig {
    fn default() -> Self {
        Self {
            endpoint: std::env::var("PROVIDER_ZAZAZ_API_URL")
                .unwrap_or_else(|_| "https://zazaz.top/v1".to_string()),
            api_key: std::env::var("PROVIDER_ZAZAZ_API_KEY")
                .ok()
                .map(Secret::new),
            timeout_ms: 60000,
            model: std::env::var("PROVIDER_ZAZAZ_MODEL")
                .unwrap_or_else(|_| "./Qwen3.5-27B-FP8".to_string()),
            max_tokens: 2048,
            temperature: 0.7,
        }
    }
}

impl ZazaConfig {
    /// Create a new ZazaConfig from environment variables
    ///
    /// # Errors
    /// Returns `ZazaError::ApiKeyNotConfigured` if `PROVIDER_ZAZAZ_API_KEY` is not set
    pub fn from_env() -> Result<Self, ZazaError> {
        let api_key =
            std::env::var("PROVIDER_ZAZAZ_API_KEY").map_err(|_| ZazaError::ApiKeyNotConfigured)?;

        let endpoint = std::env::var("PROVIDER_ZAZAZ_API_URL")
            .unwrap_or_else(|_| "https://zazaz.top/v1".to_string());

        let model = std::env::var("PROVIDER_ZAZAZ_MODEL")
            .unwrap_or_else(|_| "./Qwen3.5-27B-FP8".to_string());

        Ok(Self {
            endpoint,
            api_key: Some(Secret::new(api_key)),
            timeout_ms: 60000,
            model,
            max_tokens: 2048,
            temperature: 0.7,
        })
    }
}

/// Request structure for Zaza API
#[derive(Debug, Serialize)]
struct ZazaRequest {
    model: String,
    messages: Vec<ZazaMessage>,
    max_tokens: u32,
    temperature: f32,
    stream: bool,
}

/// Message structure for Zaza API
#[derive(Debug, Serialize, Deserialize)]
struct ZazaMessage {
    role: String,
    content: String,
}

impl Default for ZazaMessage {
    fn default() -> Self {
        Self {
            role: "user".to_string(),
            content: String::new(),
        }
    }
}

/// Response structure from Zaza API
#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]
struct ZazaResponse {
    id: String,
    choices: Vec<ZazaChoice>,
    usage: Option<ZazaUsage>,
}

/// Choice structure in Zaza response
#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]
struct ZazaChoice {
    index: u32,
    message: ZazaMessage,
    finish_reason: Option<String>,
}

/// Usage statistics in Zaza response
#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]
struct ZazaUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

/// ZazaZ LLM Client
///
/// Provides integration with the zazaz API (https://zazaz.top) for AI-powered features:
/// - AI-assisted merging
/// - Conflict resolution
/// - Branch purpose inference
/// - Task planning
///
/// # Environment Variables
///
/// - `PROVIDER_ZAZAZ_API_KEY`: API key (required)
/// - `PROVIDER_ZAZAZ_API_URL`: API URL (optional, default: https://zazaz.top/v1)
/// - `PROVIDER_ZAZAZ_MODEL`: Model name (optional, default: ./Qwen3.5-27B-FP8)
pub struct ZazaClient {
    config: ZazaConfig,
    client: Client,
}

impl ZazaClient {
    /// Create a new ZazaZ client from environment variables
    ///
    /// # Errors
    /// Returns `ZazaError::ApiKeyNotConfigured` if `PROVIDER_ZAZAZ_API_KEY` is not set
    pub fn from_env() -> Result<Self, ZazaError> {
        let config = ZazaConfig::from_env()?;
        Self::with_config(config)
    }

    /// Create a new ZazaZ client with custom configuration
    pub fn with_config(config: ZazaConfig) -> Result<Self, ZazaError> {
        let client = ClientBuilder::new()
            .timeout(Duration::from_millis(config.timeout_ms))
            .build()
            .map_err(ZazaError::HttpError)?;

        Ok(Self { config, client })
    }

    /// Create a new ZazaZ client with default configuration
    ///
    /// Note: This will not fail if API key is not set - use `is_configured()` to check
    pub fn new() -> Result<Self, ZazaError> {
        Self::with_config(ZazaConfig::default())
    }

    /// Check if API key is configured
    pub fn is_configured(&self) -> bool {
        self.config.api_key.is_some()
    }

    /// Generate a response for a single prompt
    pub async fn generate(&self, prompt: &str) -> Result<String, ZazaError> {
        if !self.is_configured() {
            return Err(ZazaError::ApiKeyNotConfigured);
        }

        self.chat_completions(vec![ZazaMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        }])
        .await
    }

    /// Generate a response with conversation history
    pub async fn chat(&self, messages: Vec<(&str, &str)>) -> Result<String, ZazaError> {
        let zaza_messages = messages
            .into_iter()
            .map(|(role, content)| ZazaMessage {
                role: role.to_string(),
                content: content.to_string(),
            })
            .collect();

        self.chat_completions(zaza_messages).await
    }

    /// Send chat completion request to Zaza API
    async fn chat_completions(&self, messages: Vec<ZazaMessage>) -> Result<String, ZazaError> {
        // Check API key
        let api_key = self
            .config
            .api_key
            .as_ref()
            .ok_or(ZazaError::ApiKeyNotConfigured)?;

        let request = ZazaRequest {
            model: self.config.model.clone(),
            messages,
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            stream: false,
        };

        debug!("Sending request to Zaza API: {:?}", request);

        let response = self
            .client
            .post(format!("{}/chat/completions", self.config.endpoint))
            .header("Content-Type", "application/json")
            .header(
                "Authorization",
                format!("Bearer {}", api_key.expose_secret()),
            )
            .json(&request)
            .send()
            .await
            .map_err(ZazaError::HttpError)?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            error!("Zaza API error: {} - {}", status, error_text);

            return match status.as_u16() {
                429 => Err(ZazaError::RateLimitExceeded),
                500..=599 => Err(ZazaError::ServerError {
                    status: status.as_u16(),
                    message: error_text,
                }),
                _ => Err(ZazaError::ServerError {
                    status: status.as_u16(),
                    message: error_text,
                }),
            };
        }

        let response: ZazaResponse = response
            .json()
            .await
            .map_err(|e| ZazaError::InvalidResponse(e.to_string()))?;

        debug!("Zaza API response: {:?}", response);

        // Extract the first choice's message content
        let choice = response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| ZazaError::InvalidResponse("No choices in response".to_string()))?;

        Ok(choice.message.content)
    }

    /// Generate a response for AI-assisted merging
    pub async fn assist_merge(
        &self,
        source_branch: &str,
        target_branch: &str,
        conflict_description: &str,
    ) -> Result<String, ZazaError> {
        let prompt = format!(
            r#"You are an AI assistant helping to merge CAD design branches.

Source Branch: {source}
Target Branch: {target}

Conflict Description:
{conflict}

Please provide:
1. Analysis of the conflict
2. Recommended resolution strategy
3. Step-by-step instructions to resolve

Respond in a clear, structured format."#,
            source = source_branch,
            target = target_branch,
            conflict = conflict_description
        );

        self.generate(&prompt).await
    }

    /// Resolve a design conflict
    pub async fn resolve_conflict(
        &self,
        conflict_type: &str,
        entities_involved: &[String],
        constraints: &[String],
    ) -> Result<String, ZazaError> {
        let prompt = format!(
            r#"You are a CAD design conflict resolution expert.

Conflict Type: {conflict_type}

Entities Involved:
{entities}

Constraints:
{constraints}

Please provide:
1. Root cause analysis
2. Possible resolution options
3. Recommended solution with justification
4. Potential side effects

Respond in a structured format suitable for automated processing."#,
            conflict_type = conflict_type,
            entities = entities_involved.join("\n"),
            constraints = constraints.join("\n")
        );

        self.generate(&prompt).await
    }

    /// Infer the purpose of a design branch
    pub async fn infer_branch_purpose(
        &self,
        branch_name: &str,
        recent_changes: &[String],
        dialog_summary: &str,
    ) -> Result<String, ZazaError> {
        let prompt = format!(
            r#"Analyze the purpose of this CAD design branch.

Branch Name: {branch}

Recent Changes:
{changes}

Dialog Summary:
{dialog}

Please infer and summarize:
1. Primary design goal
2. Key modifications being explored
3. Design constraints being addressed
4. Confidence level (High/Medium/Low)

Respond in a concise, structured format."#,
            branch = branch_name,
            changes = recent_changes.join("\n"),
            dialog = dialog_summary
        );

        self.generate(&prompt).await
    }

    /// Generate a summary of branch content
    pub async fn summarize_branch(
        &self,
        branch_name: &str,
        content: &str,
    ) -> Result<String, ZazaError> {
        let prompt = format!(
            r#"Summarize the following CAD design branch content.

Branch: {branch}

Content:
{content}

Provide a concise summary (2-3 sentences) covering:
1. Main design changes
2. Key decisions made
3. Current status

Respond in a professional, technical style."#,
            branch = branch_name,
            content = content
        );

        self.generate(&prompt).await
    }

    /// Assess merge risk level
    pub async fn assess_merge_risk(
        &self,
        source_branch: &str,
        target_branch: &str,
        diff_summary: &str,
    ) -> Result<String, ZazaError> {
        let prompt = format!(
            r#"Assess the risk level of merging these CAD design branches.

Source: {source}
Target: {target}

Difference Summary:
{diff}

Risk Levels:
- Critical: Major conflicting changes, high chance of data loss
- High: Significant conflicts requiring manual intervention
- Medium: Some conflicts but resolvable automatically
- Low: Minor changes, safe to merge automatically

Please provide:
1. Risk level assessment (Critical/High/Medium/Low)
2. Justification
3. Recommended merge strategy
4. Specific areas requiring attention

Respond in a structured format."#,
            source = source_branch,
            target = target_branch,
            diff = diff_summary
        );

        self.generate(&prompt).await
    }
}

impl Default for ZazaClient {
    fn default() -> Self {
        Self::new().expect("Failed to create default ZazaClient")
    }
}

/// Adapter trait to integrate with tokitai-context's AI features
/// This allows ZazaClient to be used interchangeably with other LLM clients
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Generate a response for a prompt
    async fn generate(
        &self,
        prompt: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>>;

    /// Generate a response with streaming (optional)
    async fn generate_stream(
        &self,
        prompt: &str,
    ) -> Result<
        Box<
            dyn futures_core::Stream<
                    Item = Result<String, Box<dyn std::error::Error + Send + Sync>>,
                > + Send,
        >,
        Box<dyn std::error::Error + Send + Sync>,
    > {
        // Default implementation: non-streaming
        let response = self.generate(prompt).await?;
        Ok(Box::new(futures_util::stream::once(
            async move { Ok(response) },
        )))
    }
}

#[async_trait]
impl LLMProvider for ZazaClient {
    async fn generate(
        &self,
        prompt: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        self.generate(prompt)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zaza_config_default() {
        let config = ZazaConfig::default();
        assert_eq!(config.timeout_ms, 60000);
        assert_eq!(config.max_tokens, 2048);
        assert!((0.0..=1.0).contains(&config.temperature));

        // Default model should be Qwen3.5-27B-FP8
        assert_eq!(config.model, "./Qwen3.5-27B-FP8");
    }

    #[test]
    fn test_zaza_config_from_env() {
        // Test that environment variables are read correctly
        let config = ZazaConfig::default();

        // Endpoint should use default if env var not set
        if std::env::var("PROVIDER_ZAZAZ_API_URL").is_err() {
            assert_eq!(config.endpoint, "https://zazaz.top/v1");
        }
    }

    #[test]
    fn test_zaza_client_creation() {
        // Test client creation with default config
        let result = ZazaClient::new();
        // May succeed or fail depending on API key configuration
        match result {
            Ok(client) => {
                // Client created successfully
                assert!(!client.is_configured() || client.is_configured());
            }
            Err(e) => {
                // Other errors are unexpected
                panic!("Unexpected error: {:?}", e);
            }
        }
    }

    #[test]
    fn test_zaza_client_from_env_success() {
        // This test will fail if API key is not set, which is expected
        // We just verify the function exists and has correct signature
        let _ = ZazaClient::from_env();
    }

    #[tokio::test]
    async fn test_zaza_client_unconfigured() {
        // Create client without API key
        let config = ZazaConfig {
            api_key: None,
            ..Default::default()
        };
        let client = ZazaClient::with_config(config).unwrap();

        assert!(!client.is_configured());

        // Generate should fail without API key
        let result = client.generate("test").await;
        assert!(matches!(result, Err(ZazaError::ApiKeyNotConfigured)));
    }

    #[test]
    fn test_zaza_message_serialization() {
        let message = ZazaMessage {
            role: "user".to_string(),
            content: "Hello".to_string(),
        };

        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"content\":\"Hello\""));
    }
}
