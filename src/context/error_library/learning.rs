//! Error Learning
//!
//! Learning strategies for error patterns and frequency analysis.

use super::types::{ErrorCase, ErrorSeverity};
use std::collections::HashMap;

/// Error frequency tracker
#[derive(Debug, Default)]
pub struct ErrorFrequencyTracker {
    /// Error type -> occurrence count
    type_counts: HashMap<String, u32>,
    /// Total occurrences
    total: u32,
}

impl ErrorFrequencyTracker {
    /// Create a new frequency tracker
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an error occurrence
    pub fn record(&mut self, error_type: &str) {
        *self.type_counts.entry(error_type.to_string()).or_insert(0) += 1;
        self.total += 1;
    }

    /// Get the frequency of an error type
    pub fn get_frequency(&self, error_type: &str) -> u32 {
        *self.type_counts.get(error_type).unwrap_or(&0)
    }

    /// Get the most frequent error types
    pub fn get_top_errors(&self, limit: usize) -> Vec<(&str, u32)> {
        let mut errors: Vec<_> = self
            .type_counts
            .iter()
            .map(|(k, v)| (k.as_str(), *v))
            .collect();

        errors.sort_by(|a, b| b.1.cmp(&a.1));
        errors.truncate(limit);
        errors
    }

    /// Get total occurrences
    pub fn total(&self) -> u32 {
        self.total
    }

    /// Get unique error types count
    pub fn unique_types(&self) -> usize {
        self.type_counts.len()
    }
}

/// Error pattern analyzer
pub struct ErrorPatternAnalyzer {
    /// Historical error cases
    cases: Vec<ErrorCase>,
}

impl ErrorPatternAnalyzer {
    /// Create a new analyzer with the given cases
    pub fn new(cases: Vec<ErrorCase>) -> Self {
        Self { cases }
    }

    /// Analyze error patterns by type
    pub fn analyze_by_type(&self) -> HashMap<String, ErrorTypeStats> {
        let mut stats: HashMap<String, ErrorTypeStats> = HashMap::new();

        for case in &self.cases {
            let entry = stats
                .entry(case.error_type.clone())
                .or_insert_with(|| ErrorTypeStats::new(&case.error_type));
            entry.add_case(case);
        }

        stats
    }

    /// Find error patterns by severity
    pub fn find_by_severity(&self, severity: ErrorSeverity) -> Vec<&ErrorCase> {
        self.cases
            .iter()
            .filter(|c| c.severity() == severity)
            .collect()
    }

    /// Get cases with high occurrence count
    pub fn get_frequent_errors(&self, min_occurrences: u32) -> Vec<&ErrorCase> {
        self.cases
            .iter()
            .filter(|c| c.occurrence_count >= min_occurrences)
            .collect()
    }

    /// Get total cases
    pub fn total_cases(&self) -> usize {
        self.cases.len()
    }
}

/// Statistics for an error type
#[derive(Debug, Clone)]
pub struct ErrorTypeStats {
    /// Error type name
    pub error_type: String,
    /// Number of unique cases
    pub case_count: usize,
    /// Total occurrences
    pub total_occurrences: u32,
    /// Average confidence
    pub avg_confidence: f32,
    /// Severity distribution
    pub severity_distribution: HashMap<ErrorSeverity, usize>,
}

impl ErrorTypeStats {
    /// Create new stats for an error type
    pub fn new(error_type: &str) -> Self {
        Self {
            error_type: error_type.to_string(),
            case_count: 0,
            total_occurrences: 0,
            avg_confidence: 0.0,
            severity_distribution: HashMap::new(),
        }
    }

    /// Add a case to the stats
    pub fn add_case(&mut self, case: &ErrorCase) {
        self.case_count += 1;
        self.total_occurrences += case.occurrence_count;

        // Update average confidence (running average)
        let n = self.case_count as f32;
        self.avg_confidence = (self.avg_confidence * (n - 1.0) + case.confidence) / n;

        // Update severity distribution
        *self
            .severity_distribution
            .entry(case.severity())
            .or_insert(0) += 1;
    }

    /// Get the most common severity
    pub fn most_common_severity(&self) -> Option<ErrorSeverity> {
        self.severity_distribution
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(severity, _)| *severity)
    }
}

/// Learning strategy for error cases
pub trait LearningStrategy {
    /// Process a new error case and update learning state
    fn process_error(&mut self, case: &ErrorCase);

    /// Get recommendations based on learned patterns
    fn get_recommendations(&self) -> Vec<String>;
}

/// Simple frequency-based learning strategy
#[derive(Debug, Default)]
pub struct FrequencyLearning {
    tracker: ErrorFrequencyTracker,
}

impl FrequencyLearning {
    pub fn new() -> Self {
        Self::default()
    }
}

impl LearningStrategy for FrequencyLearning {
    fn process_error(&mut self, case: &ErrorCase) {
        self.tracker.record(&case.error_type);
    }

    fn get_recommendations(&self) -> Vec<String> {
        let top = self.tracker.get_top_errors(3);
        top.iter()
            .map(|(error_type, count)| {
                format!(
                    "Error type '{}' occurs frequently ({} times). Consider reviewing prevention measures.",
                    error_type, count
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::ErrorCase;
    use super::*;

    #[test]
    fn test_frequency_tracker() {
        let mut tracker = ErrorFrequencyTracker::new();

        tracker.record("constraint_error");
        tracker.record("constraint_error");
        tracker.record("geometry_error");

        assert_eq!(tracker.get_frequency("constraint_error"), 2);
        assert_eq!(tracker.get_frequency("geometry_error"), 1);
        assert_eq!(tracker.total(), 3);
        assert_eq!(tracker.unique_types(), 2);
    }

    #[test]
    fn test_get_top_errors() {
        let mut tracker = ErrorFrequencyTracker::new();

        for _ in 0..10 {
            tracker.record("error1");
        }
        for _ in 0..5 {
            tracker.record("error2");
        }
        for _ in 0..15 {
            tracker.record("error3");
        }

        let top = tracker.get_top_errors(2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].0, "error3");
        assert_eq!(top[0].1, 15);
        assert_eq!(top[1].0, "error1");
        assert_eq!(top[1].1, 10);
    }

    #[test]
    fn test_error_type_stats() {
        let mut stats = ErrorTypeStats::new("test_error");

        let case1 = ErrorCase::new("test_error", "Case 1", "", "", "");
        let mut case2 = ErrorCase::new("test_error", "Case 2", "", "", "");
        case2.occurrence_count = 5;
        case2.confidence = 0.8;

        stats.add_case(&case1);
        stats.add_case(&case2);

        assert_eq!(stats.case_count, 2);
        assert_eq!(stats.total_occurrences, 6);
        assert!((stats.avg_confidence - 0.9).abs() < 0.01);
    }

    #[test]
    fn test_frequency_learning() {
        let mut learning = FrequencyLearning::new();

        let case = ErrorCase::new("test_error", "", "", "", "");
        learning.process_error(&case);

        let recommendations = learning.get_recommendations();
        assert!(!recommendations.is_empty());
    }
}
