//! Error Case Types
//!
//! Core data structures for error case representation.

use serde::{Deserialize, Serialize};

/// Error version for tracking changes over time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorVersion {
    /// Version number (starts at 1)
    pub version: u32,
    /// Error case at this version
    pub case: ErrorCase,
    /// Timestamp when this version was created
    pub created_at: u64,
    /// Change description
    pub change_notes: Option<String>,
}

/// Error case representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorCase {
    /// Unique error ID
    pub id: String,
    /// Error type/category
    pub error_type: String,
    /// Error description
    pub description: String,
    /// Triggering scenario
    pub trigger_scenario: String,
    /// Root cause analysis
    pub root_cause: String,
    /// Solution or workaround
    pub solution: String,
    /// Prevention measures
    pub prevention: String,
    /// Related tools that may have caused the error
    pub related_tools: Vec<String>,
    /// Confidence score (0.0-1.0)
    pub confidence: f32,
    /// Number of times this error has occurred
    pub occurrence_count: u32,
    /// Unix timestamp when first observed
    pub first_seen: u64,
    /// Unix timestamp when last observed
    pub last_seen: u64,
    /// Tags for categorization
    pub tags: Vec<String>,
}

impl ErrorCase {
    /// Create a new error case
    pub fn new(
        error_type: &str,
        description: &str,
        trigger_scenario: &str,
        root_cause: &str,
        solution: &str,
    ) -> Self {
        let now = crate::context::utils::current_timestamp();

        Self {
            id: crate::context::utils::generate_id(),
            error_type: error_type.to_string(),
            description: description.to_string(),
            trigger_scenario: trigger_scenario.to_string(),
            root_cause: root_cause.to_string(),
            solution: solution.to_string(),
            prevention: String::new(),
            related_tools: Vec::new(),
            confidence: 1.0,
            occurrence_count: 1,
            first_seen: now,
            last_seen: now,
            tags: Vec::new(),
        }
    }

    /// Set prevention measures
    pub fn with_prevention(mut self, prevention: &str) -> Self {
        self.prevention = prevention.to_string();
        self
    }

    /// Add related tools
    pub fn with_tools(mut self, tools: Vec<&str>) -> Self {
        self.related_tools = tools.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Add tags
    pub fn with_tags(mut self, tags: Vec<&str>) -> Self {
        self.tags = tags.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Set confidence score
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Record an occurrence of this error
    pub fn record_occurrence(&mut self) {
        self.occurrence_count += 1;
        self.last_seen = crate::context::utils::current_timestamp();
    }

    /// Get the error severity based on occurrence count and confidence
    pub fn severity(&self) -> ErrorSeverity {
        if self.occurrence_count >= 10 || self.confidence < 0.5 {
            ErrorSeverity::High
        } else if self.occurrence_count >= 5 || self.confidence < 0.7 {
            ErrorSeverity::Medium
        } else {
            ErrorSeverity::Low
        }
    }
}

/// Error severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
}

impl std::fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorSeverity::Low => write!(f, "Low"),
            ErrorSeverity::Medium => write!(f, "Medium"),
            ErrorSeverity::High => write!(f, "High"),
        }
    }
}

/// Version comparison result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionComparison {
    /// Error case ID
    pub error_id: String,
    /// From version number
    pub from_version: u32,
    /// To version number
    pub to_version: u32,
    /// Fields that changed
    pub changed_fields: Vec<String>,
    /// Timestamp of the from version
    pub from_timestamp: u64,
    /// Timestamp of the to version
    pub to_timestamp: u64,
}

/// Statistics about the error library
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ErrorLibraryStats {
    /// Total number of error cases
    pub total_cases: usize,
    /// Total number of occurrences across all cases
    pub total_occurrences: u32,
    /// Number of high severity errors
    pub high_severity_count: usize,
    /// Number of medium severity errors
    pub medium_severity_count: usize,
    /// Number of low severity errors
    pub low_severity_count: usize,
    /// Unique error types
    pub error_types: Vec<String>,
}

impl std::fmt::Display for ErrorLibraryStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Error Library Statistics:")?;
        writeln!(f, "  Total cases: {}", self.total_cases)?;
        writeln!(f, "  Total occurrences: {}", self.total_occurrences)?;
        writeln!(f, "  High severity: {}", self.high_severity_count)?;
        writeln!(f, "  Medium severity: {}", self.medium_severity_count)?;
        writeln!(f, "  Low severity: {}", self.low_severity_count)?;
        writeln!(f, "  Error types: {}", self.error_types.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_case_creation() {
        let case = ErrorCase::new(
            "test_error",
            "Test description",
            "Test scenario",
            "Test root cause",
            "Test solution",
        );

        assert_eq!(case.error_type, "test_error");
        assert_eq!(case.confidence, 1.0);
        assert_eq!(case.occurrence_count, 1);
    }

    #[test]
    fn test_error_case_with_prevention() {
        let case = ErrorCase::new("error", "", "", "", "").with_prevention("Prevention measure");

        assert_eq!(case.prevention, "Prevention measure");
    }

    #[test]
    fn test_error_case_with_tags() {
        let case = ErrorCase::new("error", "", "", "", "").with_tags(vec!["critical", "geometry"]);

        assert!(case.tags.contains(&"critical".to_string()));
        assert!(case.tags.contains(&"geometry".to_string()));
    }

    #[test]
    fn test_error_severity_low() {
        let case = ErrorCase::new("error", "", "", "", "");
        assert_eq!(case.severity(), ErrorSeverity::Low);
    }

    #[test]
    fn test_error_severity_medium() {
        let mut case = ErrorCase::new("error", "", "", "", "");
        case.occurrence_count = 5;
        assert_eq!(case.severity(), ErrorSeverity::Medium);
    }

    #[test]
    fn test_error_severity_high() {
        let mut case = ErrorCase::new("error", "", "", "", "");
        case.occurrence_count = 10;
        assert_eq!(case.severity(), ErrorSeverity::High);
    }
}
