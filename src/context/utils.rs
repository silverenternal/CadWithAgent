//! Context utilities for CadAgent
//!
//! Provides common utilities used across context modules.

use crate::error::{CadAgentError, CadAgentResult};

/// Get current Unix timestamp
///
/// # Example
///
/// ```rust
/// use cadagent::context::utils::current_timestamp;
///
/// let ts = current_timestamp();
/// assert!(ts > 0);
/// ```
pub fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs()
}

/// Generate a unique ID using UUID v4
///
/// # Example
///
/// ```rust
/// use cadagent::context::utils::generate_id;
///
/// let id = generate_id();
/// assert!(!id.is_empty());
/// ```
pub fn generate_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Validate branch name format
///
/// Branch names must:
/// - Be non-empty
/// - Not contain spaces
/// - Not contain special characters except hyphens and underscores
///
/// # Arguments
///
/// * `name` - Branch name to validate
///
/// # Errors
///
/// Returns `CadAgentError::Validation` if name is invalid
pub fn validate_branch_name(name: &str) -> CadAgentResult<()> {
    if name.is_empty() {
        return Err(CadAgentError::validation(
            "Branch name cannot be empty".to_string(),
            vec!["empty_name".to_string()],
        ));
    }

    if name.contains(' ') {
        return Err(CadAgentError::validation(
            "Branch name cannot contain spaces".to_string(),
            vec!["invalid_char:space".to_string()],
        ));
    }

    // Allow only alphanumeric, hyphens, and underscores
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(CadAgentError::validation(
            "Branch name can only contain alphanumeric characters, hyphens, and underscores"
                .to_string(),
            vec!["invalid_chars".to_string()],
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_timestamp() {
        let ts = current_timestamp();
        assert!(ts > 0);
    }

    #[test]
    fn test_generate_id() {
        let id1 = generate_id();
        let id2 = generate_id();
        assert!(!id1.is_empty());
        assert!(!id2.is_empty());
        assert_ne!(id1, id2); // UUIDs should be unique
    }

    #[test]
    fn test_validate_branch_name_valid() {
        assert!(validate_branch_name("main").is_ok());
        assert!(validate_branch_name("feature-1").is_ok());
        assert!(validate_branch_name("design_option_a").is_ok());
        assert!(validate_branch_name("test123").is_ok());
    }

    #[test]
    fn test_validate_branch_name_invalid() {
        assert!(validate_branch_name("").is_err());
        assert!(validate_branch_name("has space").is_err());
        assert!(validate_branch_name("has/slash").is_err());
        assert!(validate_branch_name("has.dot").is_err());
    }
}
