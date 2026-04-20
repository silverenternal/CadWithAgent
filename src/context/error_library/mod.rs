//! Error Case Library
//!
//! Persistent storage of error patterns and solutions using tokitai-context's
//! LongTerm layer. Enables self-reflection and continuous learning from errors.
//!
//! # Features
//!
//! - **Persistent storage**: Errors are stored in LongTerm layer for permanent retention
//! - **Semantic search**: Find similar error cases using SimHash-based retrieval
//! - **Occurrence tracking**: Track how often each error occurs
//! - **Solution suggestions**: Store and retrieve solutions for common errors
//! - **Automatic learning**: Auto-record errors from tool execution
//! - **Root cause analysis**: LLM-assisted error analysis
//! - **Similar case recommendation**: Find and recommend solutions from historical cases

pub mod learning;
pub mod manager;
pub mod query;
pub mod recommender;
pub mod types;

// Re-export main types for backward compatibility
pub use learning::{
    ErrorFrequencyTracker, ErrorPatternAnalyzer, ErrorTypeStats, FrequencyLearning,
    LearningStrategy,
};
pub use manager::{
    ErrorLearningConfig, ErrorLearningManager, ErrorLearningStats, ErrorSource, LearningResult,
    SimilarCase,
};
pub use query::{ErrorCaseLibrary, ErrorLibraryConfig};
pub use recommender::{
    ErrorCaseRecommender, Recommendation, RecommendationFeedback, RecommendationStats,
    RecommenderConfig,
};
pub use types::{ErrorCase, ErrorLibraryStats, ErrorSeverity, ErrorVersion, VersionComparison};
