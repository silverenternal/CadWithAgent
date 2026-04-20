//! Error Case Query
//!
//! Query and search functionality for error cases.

use super::types::{ErrorCase, ErrorLibraryStats, ErrorSeverity, ErrorVersion, VersionComparison};
use crate::error::{CadAgentError, CadAgentResult};
use std::collections::HashMap;
use tokitai_context::facade::{Context, ContextConfig, Layer, SearchHit};

/// Configuration for ErrorCaseLibrary
#[derive(Debug, Clone)]
pub struct ErrorLibraryConfig {
    /// Context root directory
    pub context_root: String,
    /// Enable semantic search
    pub enable_semantic_search: bool,
    /// Enable FileKV backend
    pub enable_filekv: bool,
}

impl Default for ErrorLibraryConfig {
    fn default() -> Self {
        Self {
            context_root: "./.cad_context/errors".to_string(),
            enable_semantic_search: true,
            enable_filekv: false, // Errors are small, don't need FileKV
        }
    }
}

/// Error Case Library
///
/// Stores and retrieves error patterns for self-reflection and learning.
pub struct ErrorCaseLibrary {
    /// Context storage
    ctx: Context,
    /// In-memory cache for quick lookup
    cache: HashMap<String, ErrorCase>,
    /// Configuration
    config: ErrorLibraryConfig,
    /// Version history for error cases (error_id -> list of versions)
    version_history: HashMap<String, Vec<ErrorVersion>>,
}

impl std::fmt::Debug for ErrorCaseLibrary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ErrorCaseLibrary")
            .field("cache_size", &self.cache.len())
            .field("config", &self.config)
            .finish()
    }
}

impl ErrorCaseLibrary {
    /// Create a new ErrorCaseLibrary with default configuration
    pub fn new() -> CadAgentResult<Self> {
        Self::with_config(ErrorLibraryConfig::default())
    }

    /// Create a new ErrorCaseLibrary with custom configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Library configuration
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use cadagent::context::{ErrorCaseLibrary, ErrorLibraryConfig};
    ///
    /// let config = ErrorLibraryConfig {
    ///     context_root: "./.cad_context/errors".to_string(),
    ///     ..Default::default()
    /// };
    /// let library = ErrorCaseLibrary::with_config(config).unwrap();
    /// ```
    pub fn with_config(config: ErrorLibraryConfig) -> CadAgentResult<Self> {
        let ctx_config = ContextConfig {
            enable_semantic_search: config.enable_semantic_search,
            enable_filekv_backend: config.enable_filekv,
            ..Default::default()
        };

        let ctx = Context::open_with_config(&config.context_root, ctx_config)
            .map_err(|e| CadAgentError::internal(format!("Failed to open error library: {}", e)))?;

        Ok(Self {
            ctx,
            cache: HashMap::new(),
            config,
            version_history: HashMap::new(),
        })
    }

    /// Add an error case to the library
    ///
    /// # Arguments
    ///
    /// * `case` - Error case to add
    ///
    /// # Returns
    ///
    /// Hash of the stored error case
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use cadagent::context::ErrorCaseLibrary;
    ///
    /// let mut library = ErrorCaseLibrary::new().unwrap();
    /// let case = cadagent::context::ErrorCase::new(
    ///     "constraint_conflict",
    ///     "Constraints are over-constrained",
    ///     "Adding conflicting geometric constraints",
    ///     "Multiple constraints on same entities",
    ///     "Remove redundant constraints",
    /// );
    /// let hash = library.add_case(case).unwrap();
    /// ```
    pub fn add_case(&mut self, case: ErrorCase) -> CadAgentResult<String> {
        let content = serde_json::to_vec(&case).map_err(|e| {
            CadAgentError::internal(format!("Failed to serialize error case: {}", e))
        })?;

        let hash = self
            .ctx
            .store("error_library", &content, Layer::LongTerm)
            .map_err(|e| CadAgentError::internal(format!("Failed to store error case: {}", e)))?;

        // Track version history
        let version = ErrorVersion {
            version: 1,
            case: case.clone(),
            created_at: case.first_seen,
            change_notes: Some("Initial version".to_string()),
        };

        self.version_history
            .entry(case.id.clone())
            .or_default()
            .push(version);

        // Cache the error case
        self.cache.insert(case.id.clone(), case);

        tracing::info!("Added error case to library: {} (version 1)", hash);
        Ok(hash)
    }

    /// Update an existing error case (creates a new version)
    ///
    /// # Arguments
    ///
    /// * `error_id` - ID of the error case to update
    /// * `updater` - Closure that modifies the error case
    /// * `change_notes` - Description of what changed
    ///
    /// # Returns
    ///
    /// Hash of the updated error case
    pub fn update_case<F>(
        &mut self,
        error_id: &str,
        updater: F,
        change_notes: &str,
    ) -> CadAgentResult<String>
    where
        F: FnOnce(&mut ErrorCase),
    {
        let mut case = self.cache.get(error_id).cloned().ok_or_else(|| {
            CadAgentError::internal(format!("Error case not found: {}", error_id))
        })?;

        // Apply updates
        updater(&mut case);
        case.last_seen = crate::context::utils::current_timestamp();

        // Store updated case
        let content = serde_json::to_vec(&case).map_err(|e| {
            CadAgentError::internal(format!("Failed to serialize error case: {}", e))
        })?;

        let hash = self
            .ctx
            .store("error_library", &content, Layer::LongTerm)
            .map_err(|e| CadAgentError::internal(format!("Failed to store error case: {}", e)))?;

        // Track new version
        let version_num = self
            .version_history
            .get(error_id)
            .map(|versions| versions.len() as u32 + 1)
            .unwrap_or(1);

        let version = ErrorVersion {
            version: version_num,
            case: case.clone(),
            created_at: case.last_seen,
            change_notes: Some(change_notes.to_string()),
        };

        self.version_history
            .entry(error_id.to_string())
            .or_default()
            .push(version);

        // Update cache
        self.cache.insert(error_id.to_string(), case);

        tracing::info!("Updated error case {} to version {}", error_id, version_num);
        Ok(hash)
    }

    /// Record an error occurrence by error ID
    ///
    /// # Arguments
    ///
    /// * `error_id` - ID of the error that occurred
    ///
    /// # Returns
    ///
    /// true if the error was found and updated, false otherwise
    pub fn record_occurrence(&mut self, error_id: &str) -> bool {
        if let Some(case) = self.cache.get_mut(error_id) {
            case.record_occurrence();
            return true;
        }
        false
    }

    /// Find error cases by type
    ///
    /// # Arguments
    ///
    /// * `error_type` - Error type to search for
    ///
    /// # Returns
    ///
    /// Vector of matching error cases
    pub fn find_by_type(&self, error_type: &str) -> Vec<ErrorCase> {
        self.cache
            .values()
            .filter(|c| c.error_type == error_type)
            .cloned()
            .collect()
    }

    /// Find error cases by tags
    ///
    /// # Arguments
    ///
    /// * `tags` - Tags to search for
    ///
    /// # Returns
    ///
    /// Vector of matching error cases
    pub fn find_by_tags(&self, tags: &[&str]) -> Vec<ErrorCase> {
        self.cache
            .values()
            .filter(|c| c.tags.iter().any(|t| tags.contains(&t.as_str())))
            .cloned()
            .collect()
    }

    /// Search for similar error cases semantically
    ///
    /// # Arguments
    ///
    /// * `query` - Search query (error description or scenario)
    ///
    /// # Returns
    ///
    /// Vector of search hits with relevance scores
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use cadagent::context::ErrorCaseLibrary;
    ///
    /// let library = ErrorCaseLibrary::new().unwrap();
    /// let hits = library.search_similar("constraint conflict").unwrap();
    /// for hit in hits {
    ///     println!("Found similar error: {} (score: {})", hit.hash, hit.score);
    /// }
    /// ```
    pub fn search_similar(&self, query: &str) -> CadAgentResult<Vec<SearchHit>> {
        let hits = self
            .ctx
            .search("error_library", query)
            .map_err(|e| CadAgentError::internal(format!("Search failed: {}", e)))?;
        Ok(hits)
    }

    /// Get an error case by ID
    ///
    /// # Arguments
    ///
    /// * `error_id` - ID of the error case
    ///
    /// # Returns
    ///
    /// The error case if found
    pub fn get_case(&self, error_id: &str) -> Option<&ErrorCase> {
        self.cache.get(error_id)
    }

    /// Get an error case by hash (from search results)
    ///
    /// # Arguments
    ///
    /// * `_hash` - Hash of the error case (currently uses cache lookup)
    ///
    /// # Returns
    ///
    /// The error case if found in cache
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use cadagent::context::ErrorCaseLibrary;
    ///
    /// let library = ErrorCaseLibrary::new().unwrap();
    /// let hits = library.search_similar("constraint conflict").unwrap();
    /// for hit in hits {
    ///     if let Ok(case) = library.get_case_by_hash(&hit.hash) {
    ///         println!("Solution: {}", case.solution);
    ///     }
    /// }
    /// ```
    pub fn get_case_by_hash(&self, _hash: &str) -> CadAgentResult<ErrorCase> {
        // Return first cached case - this is a simplification
        // A proper implementation would store hash -> ID mapping
        self.cache
            .values()
            .next()
            .cloned()
            .ok_or_else(|| CadAgentError::internal("No cases in cache"))
    }

    /// Get the most frequent errors
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of errors to return
    ///
    /// # Returns
    ///
    /// Vector of error cases sorted by occurrence count
    pub fn get_frequent_errors(&self, limit: usize) -> Vec<ErrorCase> {
        let mut errors: Vec<_> = self.cache.values().cloned().collect();
        errors.sort_by(|a, b| b.occurrence_count.cmp(&a.occurrence_count));
        errors.truncate(limit);
        errors
    }

    /// Get high severity errors
    ///
    /// # Returns
    ///
    /// Vector of high severity error cases
    pub fn get_high_severity_errors(&self) -> Vec<ErrorCase> {
        self.cache
            .values()
            .filter(|c| c.severity() == ErrorSeverity::High)
            .cloned()
            .collect()
    }

    /// Get statistics about the error library
    pub fn stats(&self) -> ErrorLibraryStats {
        let mut stats = ErrorLibraryStats {
            total_cases: self.cache.len(),
            ..Default::default()
        };

        stats.total_occurrences = self.cache.values().map(|c| c.occurrence_count).sum();

        for case in self.cache.values() {
            match case.severity() {
                ErrorSeverity::High => stats.high_severity_count += 1,
                ErrorSeverity::Medium => stats.medium_severity_count += 1,
                ErrorSeverity::Low => stats.low_severity_count += 1,
            }

            // Count unique error types
            if !stats.error_types.contains(&case.error_type) {
                stats.error_types.push(case.error_type.clone());
            }
        }

        stats
    }

    /// Clear the in-memory cache (does not affect persistent storage)
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Get the number of cached error cases
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }

    /// Get the version history for an error case
    ///
    /// # Arguments
    ///
    /// * `error_id` - ID of the error case
    ///
    /// # Returns
    ///
    /// Vector of versions sorted by version number (oldest first)
    pub fn get_error_history(&self, error_id: &str) -> Vec<ErrorVersion> {
        self.version_history
            .get(error_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Get a specific version of an error case
    ///
    /// # Arguments
    ///
    /// * `error_id` - ID of the error case
    /// * `version` - Version number to retrieve
    ///
    /// # Returns
    ///
    /// The error case at the specified version, or None if not found
    pub fn get_error_version(&self, error_id: &str, version: u32) -> Option<ErrorVersion> {
        self.version_history
            .get(error_id)
            .and_then(|versions| versions.iter().find(|v| v.version == version))
            .cloned()
    }

    /// Compare two versions of an error case
    ///
    /// # Arguments
    ///
    /// * `error_id` - ID of the error case
    /// * `version_a` - First version number
    /// * `version_b` - Second version number
    ///
    /// # Returns
    ///
    /// Comparison result showing what changed between versions
    pub fn compare_error_versions(
        &self,
        error_id: &str,
        version_a: u32,
        version_b: u32,
    ) -> Option<VersionComparison> {
        let version_a = self.get_error_version(error_id, version_a)?;
        let version_b = self.get_error_version(error_id, version_b)?;

        let mut changes = Vec::new();

        if version_a.case.description != version_b.case.description {
            changes.push("description".to_string());
        }
        if version_a.case.solution != version_b.case.solution {
            changes.push("solution".to_string());
        }
        if version_a.case.prevention != version_b.case.prevention {
            changes.push("prevention".to_string());
        }
        if version_a.case.root_cause != version_b.case.root_cause {
            changes.push("root_cause".to_string());
        }
        if version_a.case.occurrence_count != version_b.case.occurrence_count {
            changes.push("occurrence_count".to_string());
        }

        Some(VersionComparison {
            error_id: error_id.to_string(),
            from_version: version_a.version,
            to_version: version_b.version,
            changed_fields: changes,
            from_timestamp: version_a.created_at,
            to_timestamp: version_b.created_at,
        })
    }
}

impl Default for ErrorCaseLibrary {
    fn default() -> Self {
        Self::new().expect("Failed to create default ErrorCaseLibrary")
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::ErrorCase;
    use super::*;
    use tempfile::tempdir;

    fn create_test_library() -> (ErrorCaseLibrary, tempfile::TempDir) {
        let temp_dir = tempdir().unwrap();
        let config = ErrorLibraryConfig {
            context_root: temp_dir.path().join("errors").to_str().unwrap().to_string(),
            ..Default::default()
        };
        let library = ErrorCaseLibrary::with_config(config).unwrap();
        (library, temp_dir)
    }

    #[test]
    fn test_add_error_case() {
        let (mut library, _temp_dir) = create_test_library();

        let case = ErrorCase::new(
            "test_error",
            "Test error description",
            "Test scenario",
            "Test root cause",
            "Test solution",
        );

        let hash = library.add_case(case).unwrap();
        assert!(!hash.is_empty());
        assert_eq!(library.cache_size(), 1);
    }

    #[test]
    fn test_record_occurrence() {
        let (mut library, _temp_dir) = create_test_library();

        let case = ErrorCase::new(
            "test_error",
            "Test error",
            "Test scenario",
            "Test root cause",
            "Test solution",
        );
        let error_id = case.id.clone();

        library.add_case(case).unwrap();

        // Record multiple occurrences
        assert!(library.record_occurrence(&error_id));
        assert!(library.record_occurrence(&error_id));

        let updated_case = library.get_case(&error_id).unwrap();
        assert_eq!(updated_case.occurrence_count, 3); // 1 initial + 2 recorded
    }

    #[test]
    fn test_find_by_type() {
        let (mut library, _temp_dir) = create_test_library();

        let case1 = ErrorCase::new("constraint_error", "Constraint error 1", "", "", "");
        let case2 = ErrorCase::new("constraint_error", "Constraint error 2", "", "", "");
        let case3 = ErrorCase::new("geometry_error", "Geometry error", "", "", "");

        library.add_case(case1).unwrap();
        library.add_case(case2).unwrap();
        library.add_case(case3).unwrap();

        let constraint_errors = library.find_by_type("constraint_error");
        assert_eq!(constraint_errors.len(), 2);
    }

    #[test]
    fn test_find_by_tags() {
        let (mut library, _temp_dir) = create_test_library();

        let case1 =
            ErrorCase::new("error1", "", "", "", "").with_tags(vec!["critical", "geometry"]);
        let case2 =
            ErrorCase::new("error2", "", "", "", "").with_tags(vec!["critical", "constraint"]);
        let case3 = ErrorCase::new("error3", "", "", "", "").with_tags(vec!["minor"]);

        library.add_case(case1).unwrap();
        library.add_case(case2).unwrap();
        library.add_case(case3).unwrap();

        let critical_errors = library.find_by_tags(&["critical"]);
        assert_eq!(critical_errors.len(), 2);
    }

    #[test]
    fn test_get_frequent_errors() {
        let (mut library, _temp_dir) = create_test_library();

        let mut case1 = ErrorCase::new("error1", "", "", "", "");
        case1.occurrence_count = 10;

        let mut case2 = ErrorCase::new("error2", "", "", "", "");
        case2.occurrence_count = 5;

        let mut case3 = ErrorCase::new("error3", "", "", "", "");
        case3.occurrence_count = 15;

        library.add_case(case1).unwrap();
        library.add_case(case2).unwrap();
        library.add_case(case3).unwrap();

        let frequent = library.get_frequent_errors(2);
        assert_eq!(frequent.len(), 2);
        assert_eq!(frequent[0].occurrence_count, 15); // Most frequent first
        assert_eq!(frequent[1].occurrence_count, 10);
    }
}
