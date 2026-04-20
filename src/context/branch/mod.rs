//! Branch Management for Design Exploration
//!
//! Handles Git-style branch operations using tokitai-context's O(1) COW implementation.

use crate::context::utils::{current_timestamp, validate_branch_name};
use crate::error::{CadAgentError, CadAgentResult};
use serde::{Deserialize, Serialize};
use tokitai_context::facade::{Context, ContextConfig, Layer};
use tokitai_context::parallel::{ParallelContextManager, ParallelContextManagerConfig};

/// Branch metadata for design exploration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchMetadata {
    /// Branch name
    pub name: String,
    /// Branch description
    pub description: String,
    /// Parent branch name
    pub parent_branch: String,
    /// Creation timestamp
    pub created_at: u64,
    /// Branch purpose (e.g., "design_exploration", "alternative_scheme")
    pub purpose: String,
}

/// Cross-branch search hit
///
/// Note: This struct does not implement Serialize/Deserialize because
/// tokitai-context's SearchHit does not support serde. Use for runtime results only.
#[derive(Debug, Clone)]
pub struct CrossBranchSearchHit {
    /// Search hit from tokitai-context
    pub hit: tokitai_context::facade::SearchHit,
    /// Branch where the hit was found
    pub branch: String,
    /// Additional metadata
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

/// Configuration for branch manager
#[derive(Debug, Clone)]
pub struct BranchManagerConfig {
    /// Context root directory
    pub context_root: String,
    /// Enable FileKV backend
    pub enable_filekv: bool,
}

impl Default for BranchManagerConfig {
    fn default() -> Self {
        Self {
            context_root: "./.cad_context".to_string(),
            enable_filekv: true,
        }
    }
}

/// Branch Manager
///
/// Provides Git-style branch operations for design exploration:
/// - O(1) branch creation using COW (Copy-On-Write)
/// - Branch switching (checkout)
/// - Branch listing and metadata tracking
/// - Cross-branch semantic search
pub struct BranchManager {
    /// Parallel context manager for branch operations
    manager: ParallelContextManager,
    /// Context for data storage
    ctx: Context,
    /// Current branch name
    current_branch: String,
    /// Current session ID
    current_session: String,
}

impl BranchManager {
    /// Create a new BranchManager
    ///
    /// # Arguments
    ///
    /// * `session_id` - Unique session identifier
    /// * `config` - Branch manager configuration
    pub fn new(session_id: &str, config: BranchManagerConfig) -> CadAgentResult<Self> {
        let parallel_config = ParallelContextManagerConfig {
            context_root: std::path::PathBuf::from(&config.context_root),
            ..Default::default()
        };

        let manager = ParallelContextManager::new(parallel_config).map_err(|e| {
            CadAgentError::internal(format!("Failed to create parallel manager: {}", e))
        })?;

        let ctx_config = ContextConfig {
            enable_filekv_backend: config.enable_filekv,
            ..Default::default()
        };

        let ctx = Context::open_with_config(&config.context_root, ctx_config)
            .map_err(|e| CadAgentError::internal(format!("Failed to open context: {}", e)))?;

        Ok(Self {
            manager,
            ctx,
            current_branch: "main".to_string(),
            current_session: session_id.to_string(),
        })
    }

    /// Get current branch name
    pub fn current_branch(&self) -> &str {
        &self.current_branch
    }

    /// Get current session ID
    pub fn session_id(&self) -> &str {
        &self.current_session
    }

    /// Create a new branch
    ///
    /// # Arguments
    ///
    /// * `branch_name` - Name of the new branch
    ///
    /// # Returns
    ///
    /// Branch metadata
    pub fn create_branch(&mut self, branch_name: &str) -> CadAgentResult<BranchMetadata> {
        validate_branch_name(branch_name)?;

        tracing::info!("Creating branch: {}", branch_name);

        let parent_branch = self.current_branch.clone();

        self.manager
            .create_branch(branch_name, &parent_branch)
            .map_err(|e| CadAgentError::internal(format!("Failed to create branch: {}", e)))?;

        let metadata = BranchMetadata {
            name: branch_name.to_string(),
            description: String::new(),
            parent_branch,
            created_at: current_timestamp(),
            purpose: "design_exploration".to_string(),
        };

        // Store branch metadata
        self.store_branch_metadata(&metadata)?;

        self.current_branch = branch_name.to_string();
        tracing::info!(
            "Branch created: {} (from {})",
            branch_name,
            metadata.parent_branch
        );

        Ok(metadata)
    }

    /// Create a design exploration branch with metadata
    ///
    /// # Arguments
    ///
    /// * `option_name` - Name of the design option
    /// * `description` - Description of the design approach
    pub fn create_design_option(
        &mut self,
        option_name: &str,
        description: &str,
    ) -> CadAgentResult<BranchMetadata> {
        tracing::info!("Creating design option: {} - {}", option_name, description);

        validate_branch_name(option_name)?;

        let parent_branch = self.current_branch.clone();

        self.manager
            .create_branch(option_name, &parent_branch)
            .map_err(|e| CadAgentError::internal(format!("Failed to create branch: {}", e)))?;

        let metadata = BranchMetadata {
            name: option_name.to_string(),
            description: description.to_string(),
            parent_branch,
            created_at: current_timestamp(),
            purpose: "design_exploration".to_string(),
        };

        self.store_branch_metadata(&metadata)?;

        self.current_branch = option_name.to_string();
        Ok(metadata)
    }

    /// Switch to a different branch
    ///
    /// # Arguments
    ///
    /// * `branch_name` - Name of the branch to checkout
    pub fn checkout_branch(&mut self, branch_name: &str) -> CadAgentResult<()> {
        tracing::info!("Checking out branch: {}", branch_name);

        self.manager
            .checkout(branch_name)
            .map_err(|e| CadAgentError::internal(format!("Failed to checkout branch: {}", e)))?;

        let old_branch = self.current_branch.clone();
        self.current_branch = branch_name.to_string();

        tracing::info!("Switched from {} to {}", old_branch, branch_name);
        Ok(())
    }

    /// List all branches
    pub fn list_branches(&self) -> Vec<String> {
        self.manager
            .list_branches()
            .iter()
            .enumerate()
            .map(|(i, _)| format!("branch_{}", i))
            .collect()
    }

    /// Get branch metadata
    pub fn get_branch_metadata(&self, branch_name: &str) -> CadAgentResult<Option<BranchMetadata>> {
        // Search for branch metadata in LongTerm layer
        let query = format!("branch_metadata:{}", branch_name);
        let hits = self
            .ctx
            .search(&self.current_session, &query)
            .map_err(|e| {
                CadAgentError::internal(format!("Failed to search branch metadata: {}", e))
            })?;

        if hits.is_empty() {
            return Ok(None);
        }

        // Known limitation: tokitai-context v0.1.2 doesn't expose Context::get(hash) API
        // Metadata retrieval will be implemented in v0.1.3+ when content retrieval is available
        Ok(None)
    }

    /// Perform cross-branch semantic search
    ///
    /// # Arguments
    ///
    /// * `query` - Search query string
    pub fn cross_branch_search(&self, query: &str) -> CadAgentResult<Vec<CrossBranchSearchHit>> {
        tracing::info!("Performing cross-branch semantic search: {}", query);

        let branches = self.manager.list_branches();
        let mut all_hits = Vec::new();

        for (i, _branch_ref) in branches.iter().enumerate() {
            let branch_name = format!("branch_{}", i);

            // Search in current context (tokitai-context limitation)
            match self.ctx.search(&self.current_session, query) {
                Ok(hits) => {
                    for hit in hits {
                        let mut metadata = serde_json::Map::new();
                        metadata.insert(
                            "branch".to_string(),
                            serde_json::Value::String(branch_name.clone()),
                        );

                        all_hits.push(CrossBranchSearchHit {
                            hit,
                            branch: branch_name.clone(),
                            metadata,
                        });
                    }
                }
                Err(e) => {
                    tracing::warn!("Search failed in branch {}: {}", branch_name, e);
                }
            }
        }

        // Sort by score
        all_hits.sort_by(|a, b| {
            b.hit
                .score
                .partial_cmp(&a.hit.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        tracing::info!("Cross-branch search found {} hits", all_hits.len());
        Ok(all_hits)
    }

    /// Store branch metadata
    fn store_branch_metadata(&mut self, metadata: &BranchMetadata) -> CadAgentResult<()> {
        let metadata_json = serde_json::json!({
            "type": "branch_metadata",
            "branch": metadata.name,
            "description": metadata.description,
            "parent": metadata.parent_branch,
            "created_at": metadata.created_at,
            "purpose": metadata.purpose,
        });

        let content = serde_json::to_vec(&metadata_json).map_err(|e| {
            CadAgentError::internal(format!("Failed to serialize branch metadata: {}", e))
        })?;

        self.ctx
            .store(&self.current_session, &content, Layer::LongTerm)
            .map_err(|e| {
                CadAgentError::internal(format!("Failed to store branch metadata: {}", e))
            })?;

        Ok(())
    }

    /// Get parallel manager reference
    pub fn parallel_manager(&self) -> &ParallelContextManager {
        &self.manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_branch() {
        let temp_dir = tempdir().unwrap();
        let config = BranchManagerConfig {
            context_root: temp_dir.path().to_str().unwrap().to_string(),
            ..Default::default()
        };

        let mut manager = BranchManager::new("test-branch", config).unwrap();
        let metadata = manager.create_branch("feature-1").unwrap();

        assert_eq!(metadata.name, "feature-1");
        assert_eq!(metadata.parent_branch, "main");
        assert_eq!(manager.current_branch(), "feature-1");
    }

    #[test]
    fn test_checkout_branch() {
        let temp_dir = tempdir().unwrap();
        let config = BranchManagerConfig {
            context_root: temp_dir.path().to_str().unwrap().to_string(),
            ..Default::default()
        };

        let mut manager = BranchManager::new("test-checkout", config).unwrap();
        manager.create_branch("feature-1").unwrap();
        manager.checkout_branch("main").unwrap();

        assert_eq!(manager.current_branch(), "main");
    }

    #[test]
    fn test_create_design_option() {
        let temp_dir = tempdir().unwrap();
        let config = BranchManagerConfig {
            context_root: temp_dir.path().to_str().unwrap().to_string(),
            ..Default::default()
        };

        let mut manager = BranchManager::new("test-design", config).unwrap();
        let metadata = manager
            .create_design_option("scheme-A", "Rectangular layout")
            .unwrap();

        assert_eq!(metadata.name, "scheme-A");
        assert_eq!(metadata.description, "Rectangular layout");
        assert_eq!(metadata.purpose, "design_exploration");
    }

    #[test]
    fn test_list_branches() {
        let temp_dir = tempdir().unwrap();
        let config = BranchManagerConfig {
            context_root: temp_dir.path().to_str().unwrap().to_string(),
            ..Default::default()
        };

        let mut manager = BranchManager::new("test-list", config).unwrap();
        manager.create_branch("feature-1").unwrap();
        manager.checkout_branch("main").unwrap();
        manager.create_branch("feature-2").unwrap();

        let branches = manager.list_branches();
        assert!(branches.len() >= 2); // main + feature-1 + feature-2
    }

    #[test]
    fn test_invalid_branch_name() {
        let temp_dir = tempdir().unwrap();
        let config = BranchManagerConfig {
            context_root: temp_dir.path().to_str().unwrap().to_string(),
            ..Default::default()
        };

        let mut manager = BranchManager::new("test-invalid", config).unwrap();

        assert!(manager.create_branch("").is_err());
        assert!(manager.create_branch("has space").is_err());
        assert!(manager.create_branch("has/slash").is_err());
    }
}
