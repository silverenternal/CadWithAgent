//! Error Case Library - Backward Compatibility Module
//!
//! This module is kept for backward compatibility.
//! Use `crate::context::error_library::*` for new code.

pub use super::error_library::learning::{
    ErrorFrequencyTracker, ErrorPatternAnalyzer, ErrorTypeStats, FrequencyLearning,
    LearningStrategy,
};
pub use super::error_library::query::{ErrorCaseLibrary, ErrorLibraryConfig};
pub use super::error_library::types::{
    ErrorCase, ErrorLibraryStats, ErrorSeverity, ErrorVersion, VersionComparison,
};

#[cfg(test)]
mod tests {
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

    #[test]
    fn test_error_severity() {
        let mut case = ErrorCase::new("error", "", "", "", "");

        // Low severity (1 occurrence, high confidence)
        assert_eq!(case.severity(), ErrorSeverity::Low);

        // Medium severity (5 occurrences)
        case.occurrence_count = 5;
        assert_eq!(case.severity(), ErrorSeverity::Medium);

        // High severity (10 occurrences)
        case.occurrence_count = 10;
        assert_eq!(case.severity(), ErrorSeverity::High);
    }

    #[test]
    fn test_stats() {
        let (mut library, _temp_dir) = create_test_library();

        let mut case1 = ErrorCase::new("error1", "", "", "", "");
        case1.occurrence_count = 10; // High severity

        let mut case2 = ErrorCase::new("error2", "", "", "", "");
        case2.occurrence_count = 5; // Medium severity

        library.add_case(case1).unwrap();
        library.add_case(case2).unwrap();

        let stats = library.stats();
        assert_eq!(stats.total_cases, 2);
        assert_eq!(stats.total_occurrences, 15);
        assert_eq!(stats.high_severity_count, 1);
        assert_eq!(stats.medium_severity_count, 1);
        assert_eq!(stats.low_severity_count, 0);
    }

    #[test]
    fn test_update_case_creates_new_version() {
        let (mut library, _temp_dir) = create_test_library();

        let case = ErrorCase::new("test_error", "Original description", "", "", "");
        let error_id = case.id.clone();

        library.add_case(case).unwrap();

        // Update the error case
        library
            .update_case(
                &error_id,
                |c| c.description = "Updated description".to_string(),
                "Updated description based on new findings",
            )
            .unwrap();

        // Should have 2 versions
        let history = library.get_error_history(&error_id);
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].version, 1);
        assert_eq!(history[1].version, 2);
    }

    #[test]
    fn test_get_error_version() {
        let (mut library, _temp_dir) = create_test_library();

        let case = ErrorCase::new("test_error", "Original", "", "", "");
        let error_id = case.id.clone();

        library.add_case(case).unwrap();

        library
            .update_case(
                &error_id,
                |c| c.description = "Updated".to_string(),
                "Update",
            )
            .unwrap();

        // Get version 1
        let v1 = library.get_error_version(&error_id, 1);
        assert!(v1.is_some());
        assert_eq!(v1.unwrap().case.description, "Original");

        // Get version 2
        let v2 = library.get_error_version(&error_id, 2);
        assert!(v2.is_some());
        assert_eq!(v2.unwrap().case.description, "Updated");
    }

    #[test]
    fn test_compare_error_versions() {
        let (mut library, _temp_dir) = create_test_library();

        let case = ErrorCase::new("test_error", "Original", "", "", "Original solution");
        let error_id = case.id.clone();

        library.add_case(case).unwrap();

        library
            .update_case(
                &error_id,
                |c| {
                    c.description = "Updated description".to_string();
                    c.solution = "Updated solution".to_string();
                },
                "Updated description and solution",
            )
            .unwrap();

        // Compare versions
        let comparison = library.compare_error_versions(&error_id, 1, 2);
        assert!(comparison.is_some());

        let comp = comparison.unwrap();
        assert_eq!(comp.from_version, 1);
        assert_eq!(comp.to_version, 2);
        assert!(comp.changed_fields.contains(&"description".to_string()));
        assert!(comp.changed_fields.contains(&"solution".to_string()));
    }

    #[test]
    fn test_version_history_empty_for_new_error() {
        let (mut library, _temp_dir) = create_test_library();

        let case = ErrorCase::new("new_error", "", "", "", "");
        let error_id = case.id.clone();

        library.add_case(case).unwrap();

        let history = library.get_error_history(&error_id);
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].version, 1);
    }
}
