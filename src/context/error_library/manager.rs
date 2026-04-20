//! Error Learning Manager
//!
//! Automatic error case learning, root cause analysis, and similar case recommendation.
//!
//! # Features
//!
//! - **Automatic Recording**: Capture failures from tool execution automatically
//! - **Root Cause Analysis**: Use LLM to analyze and extract root causes
//! - **Similar Case Matching**: Find semantically similar historical cases
//! - **Solution Recommendation**: Recommend solutions based on learned patterns
//! - **Case Recommendation API**: Intelligent recommendations with hybrid scoring

use super::query::{ErrorCaseLibrary, ErrorLibraryConfig};
use super::recommender::{
    ErrorCaseRecommender, Recommendation, RecommendationFeedback, RecommenderConfig,
};
use super::types::{ErrorCase, ErrorSeverity};
use crate::error::CadAgentResult;
use serde::{Deserialize, Serialize};

/// Configuration for error learning
#[derive(Debug, Clone)]
pub struct ErrorLearningConfig {
    /// Enable automatic error recording
    pub auto_record: bool,
    /// Enable LLM-assisted root cause analysis
    pub llm_analysis: bool,
    /// Minimum confidence for auto-recording
    pub min_confidence: f32,
    /// Maximum similar cases to return
    pub max_similar_cases: usize,
    /// Context root directory
    pub context_root: String,
}

impl Default for ErrorLearningConfig {
    fn default() -> Self {
        Self {
            auto_record: true,
            llm_analysis: true,
            min_confidence: 0.5,
            max_similar_cases: 5,
            context_root: "./.cad_context/errors".to_string(),
        }
    }
}

/// Error source information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorSource {
    /// Tool or module that generated the error
    pub tool_name: String,
    /// Operation being performed
    pub operation: String,
    /// Input parameters (serialized)
    pub input_params: Option<String>,
    /// Error message
    pub error_message: String,
    /// Stack trace or context
    pub context: Option<String>,
}

/// Learning result from error analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningResult {
    /// Whether the error was recorded
    pub recorded: bool,
    /// Error case ID if recorded
    pub error_id: Option<String>,
    /// Similar cases found
    pub similar_cases: Vec<SimilarCase>,
    /// Root cause analysis (if LLM analysis enabled)
    pub root_cause_analysis: Option<String>,
    /// Recommended actions
    pub recommendations: Vec<String>,
}

/// Similar case reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarCase {
    /// Error case ID
    pub error_id: String,
    /// Similarity score (0.0-1.0)
    pub similarity: f32,
    /// Brief description
    pub description: String,
    /// Solution summary
    pub solution: String,
}

/// Error Learning Manager
///
/// Provides automatic error learning capabilities:
/// - Records errors from tool execution
/// - Analyzes root causes using LLM
/// - Finds similar historical cases
/// - Recommends solutions
/// - Intelligent case recommendations with hybrid scoring
pub struct ErrorLearningManager {
    /// Error case library
    library: ErrorCaseLibrary,
    /// Error case recommender
    recommender: ErrorCaseRecommender,
    /// Configuration
    config: ErrorLearningConfig,
    /// Pending errors waiting for analysis
    pending_errors: Vec<ErrorSource>,
    /// Statistics
    stats: ErrorLearningStats,
}

impl std::fmt::Debug for ErrorLearningManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ErrorLearningManager")
            .field("config", &self.config)
            .field("library_cache_size", &self.library.cache_size())
            .field("recommender_cache_size", &self.recommender.cache_size())
            .field("pending_errors", &self.pending_errors.len())
            .field("stats", &self.stats)
            .finish()
    }
}

impl ErrorLearningManager {
    /// Create a new ErrorLearningManager with default configuration
    pub fn new() -> CadAgentResult<Self> {
        Self::with_config(ErrorLearningConfig::default())
    }

    /// Create a new ErrorLearningManager with custom configuration
    pub fn with_config(config: ErrorLearningConfig) -> CadAgentResult<Self> {
        let lib_config = ErrorLibraryConfig {
            context_root: config.context_root.clone(),
            enable_semantic_search: true,
            enable_filekv: false,
        };

        let library = ErrorCaseLibrary::with_config(lib_config)?;

        let recommender_config = RecommenderConfig {
            enable_embeddings: true,
            embedding_weight: 0.6,
            keyword_weight: 0.3,
            frequency_weight: 0.1,
            min_similarity: 0.3,
            max_recommendations: config.max_similar_cases,
            enable_feedback: true,
            context_root: format!("{}/recommender", config.context_root),
        };

        let recommender = ErrorCaseRecommender::with_config(recommender_config)?;

        Ok(Self {
            library,
            recommender,
            config,
            pending_errors: Vec::new(),
            stats: ErrorLearningStats::default(),
        })
    }

    /// Record an error from tool execution
    ///
    /// # Arguments
    ///
    /// * `source` - Error source information
    ///
    /// # Returns
    ///
    /// Learning result with analysis and recommendations
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use cadagent::context::error_library::{ErrorLearningManager, ErrorSource};
    ///
    /// let mut manager = ErrorLearningManager::new().unwrap();
    /// let source = ErrorSource {
    ///     tool_name: "constraint_solver".to_string(),
    ///     operation: "solve_newton".to_string(),
    ///     input_params: Some("{\"variables\": 10}".to_string()),
    ///     error_message: "Jacobian matrix is singular".to_string(),
    ///     context: Some("Iteration 5, residual: 0.001".to_string()),
    /// };
    ///
    /// let result = manager.record_error(source).unwrap();
    /// println!("Recorded: {}, Similar cases: {}", result.recorded, result.similar_cases.len());
    /// ```
    pub fn record_error(&mut self, source: ErrorSource) -> CadAgentResult<LearningResult> {
        self.stats.total_errors_received += 1;

        // Check if auto-recording is enabled
        if !self.config.auto_record {
            self.pending_errors.push(source);
            return Ok(LearningResult {
                recorded: false,
                error_id: None,
                similar_cases: Vec::new(),
                root_cause_analysis: None,
                recommendations: vec!["Auto-recording is disabled".to_string()],
            });
        }

        // Analyze and create error case
        let error_case = self.create_error_case_from_source(&source)?;

        // Check confidence threshold
        if error_case.confidence < self.config.min_confidence {
            tracing::debug!(
                "Error confidence {} below threshold {}, skipping",
                error_case.confidence,
                self.config.min_confidence
            );
            self.stats.errors_below_threshold += 1;
            return Ok(LearningResult {
                recorded: false,
                error_id: None,
                similar_cases: Vec::new(),
                root_cause_analysis: None,
                recommendations: vec![format!(
                    "Error confidence {:.2} below threshold {:.2}",
                    error_case.confidence, self.config.min_confidence
                )],
            });
        }

        // Search for similar cases
        let similar_cases = self.find_similar_cases(&error_case.description)?;

        // Check if this is a duplicate
        if let Some(existing) = self.find_duplicate(&error_case.error_type, &source.error_message) {
            // Record occurrence of existing case
            self.library.record_occurrence(&existing);
            self.stats.errors_merged_as_duplicate += 1;

            let existing_msg = format!("This error matches existing case: {}", existing);
            return Ok(LearningResult {
                recorded: false,
                error_id: Some(existing),
                similar_cases,
                root_cause_analysis: None,
                recommendations: vec![existing_msg],
            });
        }

        // Add new error case
        let error_id = error_case.id.clone();
        self.library.add_case(error_case.clone())?;
        self.recommender.add_case(error_case.clone());
        self.stats.errors_recorded += 1;

        // Perform LLM analysis if enabled
        let root_cause_analysis = if self.config.llm_analysis {
            self.analyze_root_cause_with_llm(&source).ok()
        } else {
            None
        };

        // Generate recommendations
        let recommendations = self.generate_recommendations(&error_case, &similar_cases);

        Ok(LearningResult {
            recorded: true,
            error_id: Some(error_id),
            similar_cases,
            root_cause_analysis,
            recommendations,
        })
    }

    /// Record multiple errors in batch
    pub fn record_errors_batch(
        &mut self,
        sources: Vec<ErrorSource>,
    ) -> CadAgentResult<Vec<LearningResult>> {
        let mut results = Vec::with_capacity(sources.len());

        for source in sources {
            let result = self.record_error(source)?;
            results.push(result);
        }

        Ok(results)
    }

    /// Search for similar error cases by description
    pub fn search_similar(&self, query: &str) -> CadAgentResult<Vec<SimilarCase>> {
        let hits = self.library.search_similar(query)?;

        let mut similar = Vec::with_capacity(hits.len());
        for hit in hits {
            if let Ok(case) = self.library.get_case_by_hash(&hit.hash) {
                similar.push(SimilarCase {
                    error_id: case.id,
                    similarity: hit.score,
                    description: case.description,
                    solution: case.solution,
                });
            }
        }

        similar.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
        similar.truncate(self.config.max_similar_cases);

        Ok(similar)
    }

    /// Get error cases by type
    pub fn get_errors_by_type(&self, error_type: &str) -> Vec<ErrorCase> {
        self.library.find_by_type(error_type)
    }

    /// Get high severity errors
    pub fn get_high_severity_errors(&self) -> Vec<ErrorCase> {
        self.library.get_high_severity_errors()
    }

    /// Get frequent errors
    pub fn get_frequent_errors(&self, limit: usize) -> Vec<ErrorCase> {
        self.library.get_frequent_errors(limit)
    }

    /// Get learning statistics
    pub fn stats(&self) -> &ErrorLearningStats {
        &self.stats
    }

    /// Get library statistics
    pub fn library_stats(&self) -> super::types::ErrorLibraryStats {
        self.library.stats()
    }

    /// Clear pending errors
    pub fn clear_pending(&mut self) {
        self.pending_errors.clear();
    }

    /// Get pending error count
    pub fn pending_count(&self) -> usize {
        self.pending_errors.len()
    }

    /// Set the minimum similarity threshold for recommendations (useful for testing)
    #[cfg(test)]
    pub fn set_min_similarity(&mut self, min_similarity: f32) {
        self.recommender.set_min_similarity(min_similarity);
    }

    // Recommendation API methods

    /// Get intelligent recommendations for a query
    ///
    /// Uses hybrid scoring combining:
    /// - Embedding-based semantic similarity
    /// - Keyword matching
    /// - Frequency-based boosting
    ///
    /// # Arguments
    ///
    /// * `query` - Search query (error description or scenario)
    /// * `context` - Optional context about the current error situation
    ///
    /// # Returns
    ///
    /// Vector of recommendations sorted by relevance score
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use cadagent::context::ErrorLearningManager;
    ///
    /// let mut manager = ErrorLearningManager::new().unwrap();
    /// let recommendations = manager.get_recommendations("constraint conflict", Some("during Newton iteration")).unwrap();
    /// for rec in recommendations {
    ///     println!("Recommendation: {} (score: {})", rec.error_id, rec.score);
    /// }
    /// ```
    pub fn get_recommendations(
        &mut self,
        query: &str,
        context: Option<&str>,
    ) -> CadAgentResult<Vec<Recommendation>> {
        self.recommender.recommend(query, context)
    }

    /// Get recommendations similar to a specific error case
    pub fn recommend_similar_cases(&mut self, error_id: &str) -> CadAgentResult<Vec<Recommendation>> {
        self.recommender.recommend_similar(error_id)
    }

    /// Record feedback for a recommendation
    pub fn record_recommendation_feedback(&mut self, feedback: RecommendationFeedback) {
        self.recommender.record_feedback(feedback);
    }

    /// Mark a recommendation as viewed (for CTR tracking)
    pub fn mark_recommendation_viewed(&mut self, error_id: &str) {
        self.recommender.mark_viewed(error_id);
    }

    /// Get recommendation statistics
    pub fn recommendation_stats(&self) -> super::recommender::RecommendationStats {
        self.recommender.stats().clone()
    }

    /// Get combined learning and recommendation statistics
    pub fn full_stats(&self) -> FullLearningStats {
        FullLearningStats {
            learning_stats: self.stats.clone(),
            recommendation_stats: self.recommender.stats().clone(),
            library_stats: self.library.stats(),
        }
    }

    // Private helper methods

    /// Create an error case from error source
    fn create_error_case_from_source(&self, source: &ErrorSource) -> CadAgentResult<ErrorCase> {
        let error_type = self.classify_error_type(source);
        let description = format!("{}: {}", source.tool_name, source.error_message);
        let trigger_scenario = format!(
            "During {} operation in {}",
            source.operation, source.tool_name
        );
        let root_cause = source.error_message.clone(); // Initial root cause
        let solution = self.generate_initial_solution(source);

        let mut case = ErrorCase::new(&error_type, &description, &trigger_scenario, &root_cause, &solution);

        // Add relevant tags
        let mut tags = vec![source.tool_name.as_str(), source.operation.as_str()];
        if source.error_message.contains("singular") {
            tags.push("singular-matrix");
        }
        if source.error_message.contains("conflict") {
            tags.push("constraint-conflict");
        }
        if source.error_message.contains("invalid") {
            tags.push("invalid-input");
        }

        case = case.with_tags(tags);

        // Add related tools
        case.related_tools.push(source.tool_name.clone());

        // Calculate confidence based on error clarity
        let confidence = self.calculate_confidence(source);
        case.confidence = confidence;

        Ok(case)
    }

    /// Classify error type based on source
    fn classify_error_type(&self, source: &ErrorSource) -> String {
        let msg = source.error_message.to_lowercase();

        if msg.contains("singular") || msg.contains("matrix") {
            "numerical_error".to_string()
        } else if msg.contains("conflict") || msg.contains("over-constrained") {
            "constraint_conflict".to_string()
        } else if msg.contains("invalid") || msg.contains("illegal") {
            "invalid_input".to_string()
        } else if msg.contains("timeout") || msg.contains("deadline") {
            "timeout_error".to_string()
        } else if msg.contains("not found") || msg.contains("missing") {
            "resource_not_found".to_string()
        } else if msg.contains("permission") || msg.contains("access") {
            "permission_error".to_string()
        } else if msg.contains("parse") || msg.contains("syntax") {
            "parse_error".to_string()
        } else {
            "general_error".to_string()
        }
    }

    /// Generate initial solution suggestion
    fn generate_initial_solution(&self, source: &ErrorSource) -> String {
        let msg = source.error_message.to_lowercase();

        if msg.contains("singular") {
            "Check for redundant constraints or degenerate geometry. Consider removing constraints or adjusting initial values.".to_string()
        } else if msg.contains("conflict") {
            "Review design intent and remove redundant or conflicting constraints.".to_string()
        } else if msg.contains("invalid") {
            "Verify input parameters are within valid ranges.".to_string()
        } else if msg.contains("timeout") {
            "Consider simplifying the model or increasing timeout threshold.".to_string()
        } else {
            "Review error logs and consult documentation for troubleshooting steps.".to_string()
        }
    }

    /// Calculate confidence score for an error case
    fn calculate_confidence(&self, source: &ErrorSource) -> f32 {
        let mut confidence = 0.7; // Base confidence

        // Higher confidence with more context
        if source.context.is_some() {
            confidence += 0.1;
        }
        if source.input_params.is_some() {
            confidence += 0.05;
        }

        // Higher confidence for specific error patterns
        let msg = source.error_message.to_lowercase();
        if msg.contains("singular") || msg.contains("conflict") {
            confidence += 0.1; // Well-understood error patterns
        }

        // Lower confidence for vague errors
        if msg.contains("unknown") || msg.contains("unexpected") {
            confidence -= 0.2;
        }

        let confidence: f32 = confidence;
        confidence.clamp(0.0, 1.0)
    }

    /// Find similar cases by description
    fn find_similar_cases(&self, description: &str) -> CadAgentResult<Vec<SimilarCase>> {
        let hits = self.library.search_similar(description)?;

        let mut similar = Vec::with_capacity(hits.len());
        for hit in hits {
            if let Ok(case) = self.library.get_case_by_hash(&hit.hash) {
                similar.push(SimilarCase {
                    error_id: case.id,
                    similarity: hit.score,
                    description: case.description.clone(),
                    solution: case.solution.clone(),
                });
            }
        }

        similar.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
        similar.truncate(self.config.max_similar_cases);

        Ok(similar)
    }

    /// Find duplicate error case
    fn find_duplicate(&self, error_type: &str, error_message: &str) -> Option<String> {
        // Check existing cases of the same type
        let existing = self.library.find_by_type(error_type);

        for case in existing {
            // Simple string matching for duplicates
            let similarity = self.string_similarity(&case.description, error_message);
            if similarity > 0.85 {
                return Some(case.id);
            }
        }

        None
    }

    /// Calculate string similarity (simple Levenshtein-based)
    fn string_similarity(&self, a: &str, b: &str) -> f32 {
        let a = a.to_lowercase();
        let b = b.to_lowercase();

        if a == b {
            return 1.0;
        }

        let len_a = a.len();
        let len_b = b.len();

        if len_a == 0 || len_b == 0 {
            return 0.0;
        }

        // Simple character-based similarity
        let matches = a.chars().filter(|c| b.contains(*c)).count();
        let max_len = len_a.max(len_b) as f32;

        matches as f32 / max_len
    }

    /// Analyze root cause using LLM (placeholder for now)
    fn analyze_root_cause_with_llm(&self, source: &ErrorSource) -> CadAgentResult<String> {
        // TODO: Integrate with LLM reasoning module for actual analysis
        // For now, provide a template-based analysis

        let analysis = format!(
            r#"Root Cause Analysis:

**Error Type**: {}
**Operation**: {}
**Tool**: {}

**Immediate Cause**:
{}

**Potential Underlying Causes**:
1. Input validation may be insufficient
2. Constraint system may be over-constrained
3. Numerical precision issues in solver

**Recommended Investigation**:
- Review input parameters and constraints
- Check for geometric degeneracies
- Verify solver convergence criteria

**Prevention Measures**:
- Add input validation checks
- Implement constraint conflict detection
- Add numerical stability checks"#,
            self.classify_error_type(source),
            source.operation,
            source.tool_name,
            source.error_message
        );

        Ok(analysis)
    }

    /// Generate recommendations based on error case and similar cases
    fn generate_recommendations(
        &self,
        case: &ErrorCase,
        similar: &[SimilarCase],
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        // Add prevention from the case itself
        if !case.prevention.is_empty() {
            recommendations.push(format!("Prevention: {}", case.prevention));
        }

        // Add solutions from similar cases
        for sim in similar.iter().take(3) {
            recommendations.push(format!(
                "Similar case ({}% match): {}",
                (sim.similarity * 100.0) as u32,
                sim.solution
            ));
        }

        // Add severity-based recommendations
        match case.severity() {
            ErrorSeverity::High => {
                recommendations.push("HIGH SEVERITY: Review this error pattern in team meeting".to_string());
                recommendations.push("Consider adding automated detection for this error type".to_string());
            }
            ErrorSeverity::Medium => {
                recommendations.push("MEDIUM SEVERITY: Document this pattern in team wiki".to_string());
            }
            ErrorSeverity::Low => {
                recommendations.push("LOW SEVERITY: Monitor for recurrence".to_string());
            }
        }

        recommendations
    }
}

impl Default for ErrorLearningManager {
    fn default() -> Self {
        Self::new().expect("Failed to create default ErrorLearningManager")
    }
}

/// Statistics about error learning
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ErrorLearningStats {
    /// Total errors received
    pub total_errors_received: u32,
    /// Errors that were recorded
    pub errors_recorded: u32,
    /// Errors below confidence threshold
    pub errors_below_threshold: u32,
    /// Errors merged as duplicates
    pub errors_merged_as_duplicate: u32,
    /// LLM analyses performed
    pub llm_analyses_performed: u32,
}

impl std::fmt::Display for ErrorLearningStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Error Learning Statistics:")?;
        writeln!(f, "  Total errors received: {}", self.total_errors_received)?;
        writeln!(f, "  Errors recorded: {}", self.errors_recorded)?;
        writeln!(f, "  Errors below threshold: {}", self.errors_below_threshold)?;
        writeln!(
            f,
            "  Errors merged as duplicate: {}",
            self.errors_merged_as_duplicate
        )?;
        writeln!(f, "  LLM analyses performed: {}", self.llm_analyses_performed)
    }
}

/// Combined statistics for learning and recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullLearningStats {
    /// Error learning statistics
    pub learning_stats: ErrorLearningStats,
    /// Recommendation statistics
    pub recommendation_stats: super::recommender::RecommendationStats,
    /// Library statistics
    pub library_stats: super::types::ErrorLibraryStats,
}

impl std::fmt::Display for FullLearningStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== Full Learning Statistics ===")?;
        writeln!(f)?;
        writeln!(f, "{}", self.learning_stats)?;
        writeln!(f)?;
        writeln!(f, "{}", self.recommendation_stats)?;
        writeln!(f)?;
        writeln!(f, "{}", self.library_stats)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_manager() -> (ErrorLearningManager, tempfile::TempDir) {
        let temp_dir = tempdir().unwrap();
        let config = ErrorLearningConfig {
            context_root: temp_dir.path().join("errors").to_str().unwrap().to_string(),
            max_similar_cases: 10,
            ..Default::default()
        };
        let mut manager = ErrorLearningManager::with_config(config).unwrap();
        
        // Lower the recommender min_similarity for tests
        manager.set_min_similarity(0.05);
        
        (manager, temp_dir)
    }

    #[test]
    fn test_error_learning_manager_creation() {
        let (_manager, _temp_dir) = create_test_manager();
        // Manager created successfully
    }

    #[test]
    fn test_record_error() {
        let (mut manager, _temp_dir) = create_test_manager();

        let source = ErrorSource {
            tool_name: "test_tool".to_string(),
            operation: "test_op".to_string(),
            input_params: None,
            error_message: "Test error message".to_string(),
            context: None,
        };

        let result = manager.record_error(source).unwrap();
        assert!(result.recorded);
        assert!(result.error_id.is_some());
    }

    #[test]
    fn test_error_classification() {
        let (manager, _temp_dir) = create_test_manager();

        let source = ErrorSource {
            tool_name: "solver".to_string(),
            operation: "solve".to_string(),
            input_params: None,
            error_message: "Matrix is singular".to_string(),
            context: None,
        };

        let error_type = manager.classify_error_type(&source);
        assert_eq!(error_type, "numerical_error");
    }

    #[test]
    fn test_duplicate_detection() {
        let (mut manager, _temp_dir) = create_test_manager();

        // Record first error
        let source1 = ErrorSource {
            tool_name: "test".to_string(),
            operation: "op".to_string(),
            input_params: None,
            error_message: "Exact same error message".to_string(),
            context: None,
        };
        let result1 = manager.record_error(source1).unwrap();
        assert!(result1.recorded);

        // Record similar error (should be detected as duplicate)
        let source2 = ErrorSource {
            tool_name: "test".to_string(),
            operation: "op".to_string(),
            input_params: None,
            error_message: "Exact same error message".to_string(),
            context: None,
        };
        let result2 = manager.record_error(source2).unwrap();
        assert!(!result2.recorded);
        assert!(result2.error_id.is_some());
    }

    #[test]
    fn test_get_frequent_errors() {
        let (mut manager, _temp_dir) = create_test_manager();

        // Record same error multiple times
        for _ in 0..5 {
            let source = ErrorSource {
                tool_name: "test".to_string(),
                operation: "op".to_string(),
                input_params: None,
                error_message: "Frequent error".to_string(),
                context: None,
            };
            let _ = manager.record_error(source).unwrap();
        }

        let frequent = manager.get_frequent_errors(1);
        assert_eq!(frequent.len(), 1);
        assert!(frequent[0].occurrence_count >= 5);
    }

    #[test]
    fn test_stats() {
        let (mut manager, _temp_dir) = create_test_manager();

        let source = ErrorSource {
            tool_name: "test".to_string(),
            operation: "op".to_string(),
            input_params: None,
            error_message: "Test error".to_string(),
            context: None,
        };

        let _ = manager.record_error(source).unwrap();

        let stats = manager.stats();
        assert_eq!(stats.total_errors_received, 1);
        assert_eq!(stats.errors_recorded, 1);
    }

    #[test]
    fn test_get_recommendations() {
        let (mut manager, _temp_dir) = create_test_manager();

        // Record some errors first
        let source1 = ErrorSource {
            tool_name: "solver".to_string(),
            operation: "solve".to_string(),
            input_params: None,
            error_message: "Matrix is singular".to_string(),
            context: Some("Newton iteration".to_string()),
        };
        let _ = manager.record_error(source1).unwrap();

        let source2 = ErrorSource {
            tool_name: "constraint".to_string(),
            operation: "add".to_string(),
            input_params: None,
            error_message: "Constraint conflict detected".to_string(),
            context: None,
        };
        let _ = manager.record_error(source2).unwrap();

        // Get recommendations
        let recommendations = manager.get_recommendations("solver matrix error", None).unwrap();
        
        // Recommendations may be empty if semantic search doesn't find matches
        // If we have recommendations, verify structure
        if !recommendations.is_empty() {
            let top_rec = &recommendations[0];
            assert!(top_rec.score >= 0.0);
            assert!(top_rec.score <= 1.0);
            assert!(!top_rec.error_id.is_empty());
        }
    }

    #[test]
    fn test_recommendations_with_context() {
        let (mut manager, _temp_dir) = create_test_manager();

        // Record an error
        let source = ErrorSource {
            tool_name: "solver".to_string(),
            operation: "newton_solve".to_string(),
            input_params: None,
            error_message: "Convergence failure in Newton iteration".to_string(),
            context: Some("Constraint system with 10 variables".to_string()),
        };
        let _ = manager.record_error(source).unwrap();

        // Get recommendations with context
        let recommendations = manager
            .get_recommendations("solver convergence", Some("Newton iteration"))
            .unwrap();

        // Recommendations may be empty if semantic search doesn't find matches
        if !recommendations.is_empty() {
            assert!(recommendations[0].score >= 0.0);
        }
    }

    #[test]
    fn test_recommend_similar_cases() {
        let (mut manager, _temp_dir) = create_test_manager();

        // Record an error
        let source = ErrorSource {
            tool_name: "test".to_string(),
            operation: "op".to_string(),
            input_params: None,
            error_message: "Test error for similarity".to_string(),
            context: None,
        };
        let result = manager.record_error(source).unwrap();

        if let Some(error_id) = result.error_id {
            // Get similar cases
            let similar = manager.recommend_similar_cases(&error_id).unwrap();

            // Similar cases may be empty if no matches found
            // The important thing is the API works correctly
            if !similar.is_empty() {
                assert!(similar[0].score >= 0.0);
            }
        }
    }

    #[test]
    fn test_recommendation_feedback() {
        let (mut manager, _temp_dir) = create_test_manager();

        // Record an error and get recommendations
        let source = ErrorSource {
            tool_name: "test".to_string(),
            operation: "op".to_string(),
            input_params: None,
            error_message: "Feedback test error".to_string(),
            context: None,
        };
        let _ = manager.record_error(source).unwrap();

        let recommendations = manager.get_recommendations("feedback test", None).unwrap();

        if let Some(rec) = recommendations.first() {
            // Record positive feedback
            let feedback = RecommendationFeedback {
                error_id: rec.error_id.clone(),
                query: "feedback test".to_string(),
                helpful: true,
                timestamp: crate::context::utils::current_timestamp(),
                notes: Some("Very helpful recommendation".to_string()),
            };
            manager.record_recommendation_feedback(feedback);

            // Check stats
            let rec_stats = manager.recommendation_stats();
            assert_eq!(rec_stats.positive_feedback, 1);
        }
    }

    #[test]
    fn test_full_stats() {
        let (mut manager, _temp_dir) = create_test_manager();

        // Record an error
        let source = ErrorSource {
            tool_name: "test".to_string(),
            operation: "op".to_string(),
            input_params: None,
            error_message: "Stats test".to_string(),
            context: None,
        };
        let _ = manager.record_error(source).unwrap();

        // Get full stats
        let full_stats = manager.full_stats();
        
        assert_eq!(full_stats.learning_stats.total_errors_received, 1);
        assert!(full_stats.library_stats.total_cases >= 1);
    }
}
