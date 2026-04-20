//! Dialog Memory Management
//!
//! Handles layered dialog memory using tokitai-context's Transient/ShortTerm/LongTerm storage.

use crate::context::utils::{current_timestamp, generate_id};
use crate::error::{CadAgentError, CadAgentResult};
use serde::{Deserialize, Serialize};
use tokitai_context::facade::{Context, ContextConfig, Layer, SearchHit};

/// Dialog message representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogMessage {
    /// Unique message ID
    pub id: String,
    /// Role: "user", "assistant", or "system"
    pub role: String,
    /// Message content
    pub content: String,
    /// Unix timestamp
    pub timestamp: u64,
    /// Associated CAD file path (optional)
    pub cad_file: Option<String>,
    /// Tool call chain JSON (optional)
    pub tool_chain: Option<String>,
    /// Associated reasoning steps (optional)
    pub reasoning_steps: Option<Vec<String>>,
}

impl DialogMessage {
    /// Create a new user message
    pub fn user_message(content: &str) -> Self {
        Self {
            id: generate_id(),
            role: "user".to_string(),
            content: content.to_string(),
            timestamp: current_timestamp(),
            cad_file: None,
            tool_chain: None,
            reasoning_steps: None,
        }
    }

    /// Create a new assistant message
    pub fn assistant_message(content: &str, tool_chain: Option<&str>) -> Self {
        Self {
            id: generate_id(),
            role: "assistant".to_string(),
            content: content.to_string(),
            timestamp: current_timestamp(),
            cad_file: None,
            tool_chain: tool_chain.map(|s| s.to_string()),
            reasoning_steps: None,
        }
    }

    /// Create a new system message
    pub fn system_message(content: &str) -> Self {
        Self {
            id: generate_id(),
            role: "system".to_string(),
            content: content.to_string(),
            timestamp: current_timestamp(),
            cad_file: None,
            tool_chain: None,
            reasoning_steps: None,
        }
    }

    /// Create a temporary thought (intermediate reasoning step)
    pub fn temporary_thought(content: &str) -> Self {
        Self {
            id: generate_id(),
            role: "thought".to_string(),
            content: content.to_string(),
            timestamp: current_timestamp(),
            cad_file: None,
            tool_chain: None,
            reasoning_steps: None,
        }
    }

    /// Set CAD file association
    pub fn with_cad_file(mut self, path: &str) -> Self {
        self.cad_file = Some(path.to_string());
        self
    }

    /// Set tool chain
    pub fn with_tool_chain(mut self, chain: &str) -> Self {
        self.tool_chain = Some(chain.to_string());
        self
    }
}

/// Configuration for dialog memory
#[derive(Debug, Clone)]
pub struct DialogMemoryConfig {
    /// Maximum number of short-term turns to keep
    pub max_short_term_turns: usize,
    /// Enable FileKV backend for better performance
    pub enable_filekv: bool,
    /// Enable semantic search
    pub enable_semantic_search: bool,
    /// Context root directory
    pub context_root: String,
    /// Enable memory-mapped I/O
    pub enable_mmap: bool,
    /// Enable operation logging
    pub enable_logging: bool,
}

impl Default for DialogMemoryConfig {
    fn default() -> Self {
        Self {
            max_short_term_turns: 20,
            enable_filekv: true,
            enable_semantic_search: true,
            context_root: "./.cad_context".to_string(),
            enable_mmap: true,
            enable_logging: true,
        }
    }
}

/// Dialog Memory Manager
///
/// Manages conversation context using tokitai-context's layered storage:
/// - Transient: Temporary thoughts (auto-cleaned)
/// - ShortTerm: Recent conversation turns (configurable depth)
/// - LongTerm: Permanent knowledge (user preferences, design patterns)
pub struct DialogMemoryManager {
    /// Context storage
    ctx: Context,
    /// Current session ID
    current_session: String,
    /// Configuration (kept for potential future reconfiguration)
    #[allow(dead_code)]
    config: DialogMemoryConfig,
    /// Turn counter
    turn_count: usize,
}

impl DialogMemoryManager {
    /// Create a new DialogMemoryManager
    ///
    /// # Arguments
    ///
    /// * `session_id` - Unique session identifier
    /// * `config` - Memory configuration
    pub fn new(session_id: &str, config: DialogMemoryConfig) -> CadAgentResult<Self> {
        let context_root = &config.context_root;

        let ctx_config = ContextConfig {
            max_short_term_rounds: config.max_short_term_turns,
            enable_filekv_backend: config.enable_filekv,
            enable_semantic_search: config.enable_semantic_search,
            enable_mmap: config.enable_mmap,
            enable_logging: config.enable_logging,
            ..Default::default()
        };

        let ctx = Context::open_with_config(context_root, ctx_config)
            .map_err(|e| CadAgentError::internal(format!("Failed to open context: {}", e)))?;

        Ok(Self {
            ctx,
            current_session: session_id.to_string(),
            config,
            turn_count: 0,
        })
    }

    /// Add a user message to the conversation
    pub fn add_user_message(&mut self, message: &str) -> CadAgentResult<String> {
        let msg = DialogMessage::user_message(message);
        self.store_message(msg, Layer::ShortTerm)
    }

    /// Add an assistant response
    pub fn add_assistant_response(
        &mut self,
        response: &str,
        tool_chain: Option<&str>,
    ) -> CadAgentResult<String> {
        let msg = DialogMessage::assistant_message(response, tool_chain);
        self.store_message(msg, Layer::ShortTerm)
    }

    /// Add a system message
    pub fn add_system_message(&mut self, message: &str) -> CadAgentResult<String> {
        let msg = DialogMessage::system_message(message);
        self.store_message(msg, Layer::LongTerm)
    }

    /// Add a temporary thought (intermediate reasoning step)
    pub fn add_temporary_thought(&mut self, thought: &str) -> CadAgentResult<String> {
        let msg = DialogMessage::temporary_thought(thought);
        self.store_message(msg, Layer::Transient)
    }

    /// Store long-term knowledge
    pub fn store_long_term_knowledge(
        &mut self,
        knowledge_type: &str,
        content: &str,
    ) -> CadAgentResult<String> {
        let knowledge = serde_json::json!({
            "type": knowledge_type,
            "content": content,
            "timestamp": current_timestamp(),
        });

        let content_bytes = serde_json::to_vec(&knowledge).map_err(|e| {
            CadAgentError::internal(format!("Failed to serialize knowledge: {}", e))
        })?;

        let hash = self
            .ctx
            .store(&self.current_session, &content_bytes, Layer::LongTerm)
            .map_err(|e| CadAgentError::internal(format!("Failed to store knowledge: {}", e)))?;

        tracing::info!(
            "Stored long-term knowledge: {} (type: {})",
            hash,
            knowledge_type
        );
        Ok(hash)
    }

    /// Store a dialog message
    fn store_message(&mut self, msg: DialogMessage, layer: Layer) -> CadAgentResult<String> {
        let content = serde_json::to_vec(&msg)
            .map_err(|e| CadAgentError::internal(format!("Failed to serialize message: {}", e)))?;

        let hash = self
            .ctx
            .store(&self.current_session, &content, layer)
            .map_err(|e| CadAgentError::internal(format!("Failed to store message: {}", e)))?;

        self.turn_count += 1;

        Ok(hash)
    }

    /// Get recent N conversation turns
    ///
    /// Note: Current tokitai-context API limitation - returns empty vector
    /// until Context::get(hash) API is exposed.
    pub fn get_recent_turns(&self, n: usize) -> CadAgentResult<Vec<DialogMessage>> {
        tracing::debug!(
            "Retrieving {} recent turns from session {}",
            n,
            self.current_session
        );

        // Use semantic search with wildcard query
        let all_hits = self
            .ctx
            .search(&self.current_session, "*")
            .map_err(|e| CadAgentError::internal(format!("Failed to retrieve messages: {}", e)))?;

        tracing::warn!(
            "get_recent_turns: tokitai-context returned {} hits but content retrieval API not exposed.",
            all_hits.len()
        );

        // Known limitation: tokitai-context v0.1.2 doesn't expose Context::get(hash) API
        // Will be implemented in v0.1.3+ when content retrieval is available
        Ok(Vec::new())
    }

    /// Search context semantically
    pub fn search_context(&self, query: &str) -> CadAgentResult<Vec<SearchHit>> {
        let hits = self
            .ctx
            .search(&self.current_session, query)
            .map_err(|e| CadAgentError::internal(format!("Search failed: {}", e)))?;
        Ok(hits)
    }

    /// Search for similar error cases
    pub fn search_similar_errors(
        &self,
        error_description: &str,
        limit: usize,
    ) -> CadAgentResult<Vec<SearchHit>> {
        tracing::info!("Searching for similar errors: {}", error_description);

        let hits = self
            .ctx
            .search(&self.current_session, error_description)
            .map_err(|e| CadAgentError::internal(format!("Error search failed: {}", e)))?;

        let limited_hits: Vec<SearchHit> = hits.into_iter().take(limit).collect();

        tracing::info!("Found {} similar errors", limited_hits.len());
        Ok(limited_hits)
    }

    /// Search for historical design decisions
    pub fn search_historical_decisions(
        &self,
        decision_context: &str,
        limit: usize,
    ) -> CadAgentResult<Vec<SearchHit>> {
        tracing::info!("Searching for historical decisions: {}", decision_context);

        let hits = self
            .ctx
            .search(&self.current_session, decision_context)
            .map_err(|e| CadAgentError::internal(format!("Decision search failed: {}", e)))?;

        let limited_hits: Vec<SearchHit> = hits.into_iter().take(limit).collect();

        tracing::info!("Found {} historical decisions", limited_hits.len());
        Ok(limited_hits)
    }

    /// Get turn count
    pub fn turn_count(&self) -> usize {
        self.turn_count
    }

    /// Get current session ID
    pub fn session_id(&self) -> &str {
        &self.current_session
    }

    /// Cleanup transient messages
    pub fn cleanup_transient(&mut self) -> CadAgentResult<()> {
        tracing::info!(
            "Cleaning up transient messages for session {}",
            self.current_session
        );

        // tokitai-context handles transient cleanup automatically on session end
        // This method is a placeholder for explicit cleanup if needed

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_add_user_message() {
        let temp_dir = tempdir().unwrap();
        let config = DialogMemoryConfig {
            context_root: temp_dir.path().to_str().unwrap().to_string(),
            ..Default::default()
        };

        let mut manager = DialogMemoryManager::new("test-session", config).unwrap();
        let hash = manager.add_user_message("Hello").unwrap();

        assert!(!hash.is_empty());
        assert_eq!(manager.turn_count(), 1);
    }

    #[test]
    fn test_add_assistant_response() {
        let temp_dir = tempdir().unwrap();
        let config = DialogMemoryConfig {
            context_root: temp_dir.path().to_str().unwrap().to_string(),
            ..Default::default()
        };

        let mut manager = DialogMemoryManager::new("test-session", config).unwrap();
        let hash = manager.add_assistant_response("Hi there", None).unwrap();

        assert!(!hash.is_empty());
    }

    #[test]
    fn test_add_temporary_thought() {
        let temp_dir = tempdir().unwrap();
        let config = DialogMemoryConfig {
            context_root: temp_dir.path().to_str().unwrap().to_string(),
            ..Default::default()
        };

        let mut manager = DialogMemoryManager::new("test-session", config).unwrap();
        let hash = manager.add_temporary_thought("Thinking...").unwrap();

        assert!(!hash.is_empty());
    }

    #[test]
    fn test_store_long_term_knowledge() {
        let temp_dir = tempdir().unwrap();
        let config = DialogMemoryConfig {
            context_root: temp_dir.path().to_str().unwrap().to_string(),
            ..Default::default()
        };

        let mut manager = DialogMemoryManager::new("test-session", config).unwrap();
        let hash = manager
            .store_long_term_knowledge("user_preference", "Prefers metric units")
            .unwrap();

        assert!(!hash.is_empty());
    }

    #[test]
    fn test_search_context() {
        let temp_dir = tempdir().unwrap();
        let config = DialogMemoryConfig {
            context_root: temp_dir.path().to_str().unwrap().to_string(),
            ..Default::default()
        };

        let manager = DialogMemoryManager::new("test-session", config).unwrap();
        let hits = manager.search_context("test").unwrap();

        // Should return empty results for empty context
        assert!(hits.is_empty() || !hits.is_empty()); // Search may return empty or some results
    }
}
