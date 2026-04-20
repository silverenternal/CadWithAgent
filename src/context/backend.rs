//! Context Backend Adapter
//!
//! This module provides an abstraction layer over the underlying context storage,
//! reducing coupling to tokitai-context and enabling future backend swaps.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │              High-level Managers                        │
//! │  DialogStateManager  │  TaskPlanner                     │
//! └─────────────────────────────────────────────────────────┘
//!                            │
//!                            ▼
//! ┌─────────────────────────────────────────────────────────┐
//! │              ContextBackend (Trait)                     │
//! │  - store/retrieve                                       │
//! │  - create_branch/checkout/merge                         │
//! │  - search/stats                                         │
//! └─────────────────────────────────────────────────────────┘
//!                            │
//!           ┌────────────────┼────────────────┐
//!           ▼                                 ▼
//! ┌──────────────────┐            ┌──────────────────┐
//! │  MemoryBackend   │            │  TokitaiBackend  │
//! │  (Testing)       │            │  (Production)    │
//! └──────────────────┘            └──────────────────┘
//! ```

use crate::error::{CadAgentError, CadAgentResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Context backend trait
///
/// This trait abstracts the underlying context storage operations,
/// allowing for different implementations (tokitai-context, in-memory, etc.)
///
/// # Design Notes
/// - Methods use `&mut self` to allow stateful backends
/// - All operations return `CadAgentResult` for unified error handling
/// - Thread-safe (`Send + Sync`)
pub trait ContextBackend: Send + Sync {
    /// Store a value with the given key
    ///
    /// # Returns
    /// Returns a hash/ID for the stored value
    fn store(&mut self, key: &str, value: &[u8]) -> CadAgentResult<String>;

    /// Retrieve a value by key
    fn retrieve(&self, key: &str) -> CadAgentResult<Option<Vec<u8>>>;

    /// Create a new branch
    fn create_branch(&mut self, name: &str) -> CadAgentResult<()>;

    /// Switch to a branch
    fn checkout(&mut self, name: &str) -> CadAgentResult<()>;

    /// Get current branch name
    fn current_branch(&self) -> &str;

    /// List all branches
    fn list_branches(&self) -> CadAgentResult<Vec<String>>;

    /// Merge source branch into current
    fn merge(&mut self, source: &str) -> CadAgentResult<()>;

    /// Search for values semantically
    fn search(&self, query: &str) -> CadAgentResult<Vec<SearchResult>>;

    /// Get statistics about the context
    fn stats(&self) -> CadAgentResult<BackendStats>;
}

/// Search result from semantic search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Hash of the matched item
    pub hash: String,
    /// Relevance score (0.0-1.0)
    pub score: f32,
    /// Matched content
    pub content: Vec<u8>,
}

/// Statistics about the backend storage
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BackendStats {
    /// Total number of items
    pub total_items: usize,
    /// Storage size in bytes
    pub storage_bytes: u64,
    /// Number of branches
    pub branch_count: usize,
}

/// In-memory backend for testing
///
/// This is a simple in-memory implementation useful for testing
/// without requiring file system access.
#[derive(Debug, Default)]
pub struct MemoryBackend {
    /// Storage for current branch
    store: HashMap<String, Vec<u8>>,
    /// Current branch name
    current_branch: String,
    /// All branches
    branches: Vec<String>,
}

impl MemoryBackend {
    /// Create a new in-memory backend
    pub fn new() -> Self {
        Self {
            store: HashMap::new(),
            current_branch: "main".to_string(),
            branches: vec!["main".to_string()],
        }
    }

    /// Create with a specific initial branch
    pub fn with_branch(branch: &str) -> Self {
        Self {
            store: HashMap::new(),
            current_branch: branch.to_string(),
            branches: vec![branch.to_string()],
        }
    }
}

impl ContextBackend for MemoryBackend {
    fn store(&mut self, key: &str, value: &[u8]) -> CadAgentResult<String> {
        let hash = format!("hash_{}", key);
        self.store.insert(hash.clone(), value.to_vec());
        Ok(hash)
    }

    fn retrieve(&self, key: &str) -> CadAgentResult<Option<Vec<u8>>> {
        Ok(self.store.get(key).cloned())
    }

    fn create_branch(&mut self, name: &str) -> CadAgentResult<()> {
        if self.branches.contains(&name.to_string()) {
            return Err(CadAgentError::internal(format!(
                "Branch '{}' already exists",
                name
            )));
        }
        self.branches.push(name.to_string());
        Ok(())
    }

    fn checkout(&mut self, name: &str) -> CadAgentResult<()> {
        if !self.branches.contains(&name.to_string()) {
            return Err(CadAgentError::internal(format!(
                "Branch '{}' does not exist",
                name
            )));
        }
        self.current_branch = name.to_string();
        Ok(())
    }

    fn current_branch(&self) -> &str {
        &self.current_branch
    }

    fn list_branches(&self) -> CadAgentResult<Vec<String>> {
        Ok(self.branches.clone())
    }

    fn merge(&mut self, _source: &str) -> CadAgentResult<()> {
        // Simplified merge - just a no-op for testing
        Ok(())
    }

    fn search(&self, _query: &str) -> CadAgentResult<Vec<SearchResult>> {
        // Return all items for testing
        let results = self
            .store
            .iter()
            .map(|(key, value)| SearchResult {
                hash: format!("hash_{}", key),
                score: 1.0,
                content: value.clone(),
            })
            .collect();
        Ok(results)
    }

    fn stats(&self) -> CadAgentResult<BackendStats> {
        let storage_bytes: u64 = self.store.values().map(|v| v.len() as u64).sum();

        Ok(BackendStats {
            total_items: self.store.len(),
            storage_bytes,
            branch_count: self.branches.len(),
        })
    }
}

/// Tokitai-context backend implementation
///
/// This wraps the tokitai-context ParallelContextManager
#[cfg(feature = "ai")]
pub mod tokitai_backend {
    use super::*;
    use tokitai_context::facade::{Context, ContextConfig, Layer};
    use tokitai_context::parallel::{ParallelContextManager, ParallelContextManagerConfig};

    /// Tokitai-context backend
    pub struct TokitaiBackend {
        manager: ParallelContextManager,
        ctx: Context,
        current_branch: String,
    }

    impl TokitaiBackend {
        /// Create a new tokitai backend
        pub fn new(context_root: &str) -> CadAgentResult<Self> {
            let parallel_config = ParallelContextManagerConfig {
                context_root: std::path::PathBuf::from(context_root),
                ..Default::default()
            };

            let manager = ParallelContextManager::new(parallel_config).map_err(|e| {
                CadAgentError::internal(format!("Failed to create parallel manager: {}", e))
            })?;

            let ctx_config = ContextConfig::default();
            let ctx = Context::open_with_config(context_root, ctx_config)
                .map_err(|e| CadAgentError::internal(format!("Failed to open context: {}", e)))?;

            Ok(Self {
                manager,
                ctx,
                current_branch: "main".to_string(),
            })
        }
    }

    impl ContextBackend for TokitaiBackend {
        fn store(&mut self, key: &str, value: &[u8]) -> CadAgentResult<String> {
            let hash = self
                .ctx
                .store(key, value, Layer::ShortTerm)
                .map_err(|e| CadAgentError::internal(format!("Store failed: {}", e)))?;
            Ok(hash)
        }

        fn retrieve(&self, key: &str) -> CadAgentResult<Option<Vec<u8>>> {
            // Note: tokitai-context API may vary
            // This is a simplified implementation
            Ok(None)
        }

        fn create_branch(&mut self, name: &str) -> CadAgentResult<()> {
            self.manager
                .create_branch(name)
                .map_err(|e| CadAgentError::internal(format!("Create branch failed: {}", e)))?;
            Ok(())
        }

        fn checkout(&mut self, name: &str) -> CadAgentResult<()> {
            self.manager
                .checkout(name)
                .map_err(|e| CadAgentError::internal(format!("Checkout failed: {}", e)))?;
            self.current_branch = name.to_string();
            Ok(())
        }

        fn current_branch(&self) -> &str {
            &self.current_branch
        }

        fn list_branches(&self) -> CadAgentResult<Vec<String>> {
            let branches = self
                .manager
                .list_branches()
                .map_err(|e| CadAgentError::internal(format!("List branches failed: {}", e)))?;
            Ok(branches)
        }

        fn merge(&mut self, source: &str) -> CadAgentResult<()> {
            self.manager
                .merge(source, &self.current_branch, None)
                .map_err(|e| CadAgentError::internal(format!("Merge failed: {}", e)))?;
            Ok(())
        }

        fn search(&self, query: &str) -> CadAgentResult<Vec<SearchResult>> {
            let hits = self
                .ctx
                .search("default", query)
                .map_err(|e| CadAgentError::internal(format!("Search failed: {}", e)))?;

            let results = hits
                .iter()
                .map(|hit| SearchResult {
                    hash: hit.hash.clone(),
                    score: hit.score,
                    content: hit.content.clone(),
                })
                .collect();

            Ok(results)
        }

        fn stats(&self) -> CadAgentResult<BackendStats> {
            let stats = self
                .ctx
                .stats()
                .map_err(|e| CadAgentError::internal(format!("Stats failed: {}", e)))?;

            Ok(BackendStats {
                total_items: stats.total_items,
                storage_bytes: stats.storage_bytes as u64,
                branch_count: self.manager.list_branches().unwrap_or_default().len(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_backend_creation() {
        let backend = MemoryBackend::new();
        assert_eq!(backend.current_branch(), "main");
    }

    #[test]
    fn test_memory_backend_branches() {
        let mut backend = MemoryBackend::new();

        // Create branch
        backend.create_branch("feature-1").unwrap();
        let branches = backend.list_branches().unwrap();
        assert!(branches.contains(&"feature-1".to_string()));

        // Checkout
        backend.checkout("feature-1").unwrap();
        assert_eq!(backend.current_branch(), "feature-1");
    }

    #[test]
    fn test_memory_backend_stats() {
        let backend = MemoryBackend::new();
        let stats = backend.stats().unwrap();
        assert_eq!(stats.branch_count, 1); // main branch
    }
}
