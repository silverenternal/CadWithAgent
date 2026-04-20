//! LOD Manager - Distance-based LOD switching
//!
//! This module provides automatic LOD level selection based on viewing distance.

use serde::{Deserialize, Serialize};

/// LOD (Level of Detail) levels
///
/// Each level represents a different mesh resolution:
/// - `High`: Original precision, suitable for close-up viewing
/// - `Medium`: 50% simplification, balanced quality/performance
/// - `Low`: 90% simplification, for distant objects or performance-critical scenarios
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum LodLevel {
    /// Original precision (100% vertices/faces)
    #[default]
    High,
    /// Medium simplification (~50% vertices/faces)
    Medium,
    /// High simplification (~10-20% vertices/faces)
    Low,
}

impl LodLevel {
    /// Get the target simplification ratio for this LOD level
    ///
    /// Returns a value between 0.0 (keep all) and 1.0 (remove all)
    /// representing the fraction of geometry to remove.
    pub fn simplification_ratio(&self) -> f64 {
        match self {
            LodLevel::High => 0.0,   // Keep 100%
            LodLevel::Medium => 0.5, // Keep 50%
            LodLevel::Low => 0.9,    // Keep 10%
        }
    }

    /// Get the target face count multiplier for this LOD level
    pub fn face_multiplier(&self) -> f64 {
        match self {
            LodLevel::High => 1.0,
            LodLevel::Medium => 0.5,
            LodLevel::Low => 0.1,
        }
    }
}

/// Configuration for LOD distance thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LodConfig {
    /// Distance threshold for switching to Medium LOD
    /// Objects beyond this distance use Medium LOD
    pub medium_distance: f64,
    /// Distance threshold for switching to Low LOD
    /// Objects beyond this distance use Low LOD
    pub low_distance: f64,
    /// Minimum distance for High LOD (objects closer than this are always High)
    pub high_distance: f64,
}

impl Default for LodConfig {
    fn default() -> Self {
        Self {
            high_distance: 10.0,
            medium_distance: 50.0,
            low_distance: 100.0,
        }
    }
}

impl LodConfig {
    /// Create custom LOD configuration
    ///
    /// # Arguments
    /// * `high_distance` - Maximum distance for High LOD
    /// * `medium_distance` - Maximum distance for Medium LOD
    /// * `low_distance` - Maximum distance for Low LOD (beyond this, still Low)
    ///
    /// # Panics
    /// Panics if distances are not in ascending order or contain invalid values
    pub fn new(high_distance: f64, medium_distance: f64, low_distance: f64) -> Self {
        assert!(
            high_distance > 0.0
                && high_distance < medium_distance
                && medium_distance < low_distance,
            "LOD distances must be positive and in ascending order: high < medium < low"
        );
        Self {
            high_distance,
            medium_distance,
            low_distance,
        }
    }
}

/// LOD Manager - Automatic LOD level selection based on viewing distance
///
/// The LOD manager calculates the appropriate detail level for objects
/// based on their distance from the camera/viewpoint.
///
/// # Examples
///
/// ```
/// use cadagent::lod::{LodManager, LodLevel};
///
/// let mut manager = LodManager::new();
///
/// // Custom distance thresholds
/// manager.set_lod_distances(5.0, 25.0, 75.0);
///
/// // Get LOD level for an object at distance 30.0
/// let lod = manager.get_lod_level(30.0);
/// assert_eq!(lod, LodLevel::Medium);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LodManager {
    /// LOD configuration
    config: LodConfig,
    /// Enable smooth LOD transitions (interpolation between levels)
    smooth_transitions: bool,
    /// Hysteresis factor to prevent LOD flickering (0.0 = no hysteresis)
    hysteresis: f64,
}

impl Default for LodManager {
    fn default() -> Self {
        Self::new()
    }
}

impl LodManager {
    /// Create a new LOD manager with default configuration
    pub fn new() -> Self {
        Self {
            config: LodConfig::default(),
            smooth_transitions: false,
            hysteresis: 0.1, // 10% hysteresis to prevent flickering
        }
    }

    /// Create LOD manager with custom configuration
    pub fn with_config(config: LodConfig) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }

    /// Set LOD distance thresholds
    ///
    /// # Arguments
    /// * `high_distance` - Maximum distance for High LOD
    /// * `medium_distance` - Maximum distance for Medium LOD
    /// * `low_distance` - Maximum distance for Low LOD
    ///
    /// # Panics
    /// Panics if distances are not in ascending order
    pub fn set_lod_distances(
        &mut self,
        high_distance: f64,
        medium_distance: f64,
        low_distance: f64,
    ) {
        self.config = LodConfig::new(high_distance, medium_distance, low_distance);
    }

    /// Get current LOD configuration
    pub fn config(&self) -> &LodConfig {
        &self.config
    }

    /// Enable or disable smooth LOD transitions
    pub fn set_smooth_transitions(&mut self, enabled: bool) {
        self.smooth_transitions = enabled;
    }

    /// Check if smooth transitions are enabled
    pub fn has_smooth_transitions(&self) -> bool {
        self.smooth_transitions
    }

    /// Set hysteresis factor to prevent LOD flickering
    ///
    /// Hysteresis adds a dead zone around LOD boundaries to prevent rapid
    /// switching when an object's distance fluctuates near a threshold.
    ///
    /// # Arguments
    /// * `factor` - Hysteresis factor (0.0 = no hysteresis, 1.0 = maximum)
    ///
    /// # Panics
    /// Panics if factor is not in [0.0, 1.0]
    pub fn set_hysteresis(&mut self, factor: f64) {
        assert!(
            (0.0..=1.0).contains(&factor),
            "Hysteresis factor must be between 0.0 and 1.0"
        );
        self.hysteresis = factor;
    }

    /// Get the appropriate LOD level for a given distance
    ///
    /// # Arguments
    /// * `distance` - Distance from viewpoint to object
    ///
    /// # Returns
    /// The recommended LOD level for the given distance
    ///
    /// # Examples
    ///
    /// ```
    /// use cadagent::lod::{LodManager, LodLevel};
    ///
    /// let manager = LodManager::new();
    ///
    /// // Close objects use High LOD
    /// assert_eq!(manager.get_lod_level(5.0), LodLevel::High);
    ///
    /// // Medium distance objects use Medium LOD
    /// assert_eq!(manager.get_lod_level(75.0), LodLevel::Medium);
    ///
    /// // Far objects use Low LOD
    /// assert_eq!(manager.get_lod_level(150.0), LodLevel::Low);
    /// ```
    #[allow(clippy::if_same_then_else)]
    pub fn get_lod_level(&self, distance: f64) -> LodLevel {
        // Apply hysteresis to prevent flickering
        let h = self.hysteresis;

        if distance < self.config.high_distance * (1.0 - h) {
            LodLevel::High
        } else if distance < self.config.medium_distance * (1.0 - h) {
            LodLevel::High
        } else if distance < self.config.medium_distance * (1.0 + h) {
            // In hysteresis zone - could go either way, default to higher quality
            LodLevel::High
        } else if distance < self.config.low_distance * (1.0 - h) {
            LodLevel::Medium
        } else if distance < self.config.low_distance * (1.0 + h) {
            // In hysteresis zone
            LodLevel::Medium
        } else {
            LodLevel::Low
        }
    }

    /// Get LOD level with explicit hysteresis state
    ///
    /// This method allows you to maintain LOD state across frames to fully
    /// utilize hysteresis. Pass the previous LOD level to get the new level.
    ///
    /// # Arguments
    /// * `distance` - Current distance from viewpoint
    /// * `previous_lod` - Previous frame's LOD level
    ///
    /// # Returns
    /// The recommended LOD level, considering hysteresis
    pub fn get_lod_level_with_state(&self, distance: f64, previous_lod: LodLevel) -> LodLevel {
        let h = self.hysteresis;

        match previous_lod {
            LodLevel::High => {
                // Only downgrade if significantly beyond threshold
                if distance > self.config.high_distance * (1.0 + h) {
                    LodLevel::Medium
                } else {
                    LodLevel::High
                }
            }
            LodLevel::Medium => {
                if distance < self.config.medium_distance * (1.0 - h) {
                    LodLevel::High
                } else if distance > self.config.low_distance * (1.0 + h) {
                    LodLevel::Low
                } else {
                    LodLevel::Medium
                }
            }
            LodLevel::Low => {
                // Only upgrade if significantly within threshold
                if distance < self.config.low_distance * (1.0 - h) {
                    LodLevel::Medium
                } else {
                    LodLevel::Low
                }
            }
        }
    }

    /// Calculate interpolated LOD factor for smooth transitions
    ///
    /// Returns a continuous value between 0.0 (High) and 1.0 (Low)
    /// that can be used for smooth LOD blending.
    ///
    /// # Arguments
    /// * `distance` - Distance from viewpoint
    ///
    /// # Returns
    /// Interpolated LOD factor (0.0 = High, 0.5 = Medium, 1.0 = Low)
    pub fn get_interpolated_factor(&self, distance: f64) -> f64 {
        if distance <= self.config.high_distance {
            0.0
        } else if distance <= self.config.medium_distance {
            // Interpolate between High (0.0) and Medium (0.5)
            let t = (distance - self.config.high_distance)
                / (self.config.medium_distance - self.config.high_distance);
            0.0 + t * 0.5
        } else if distance <= self.config.low_distance {
            // Interpolate between Medium (0.5) and Low (1.0)
            let t = (distance - self.config.medium_distance)
                / (self.config.low_distance - self.config.medium_distance);
            0.5 + t * 0.5
        } else {
            1.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lod_level_simplification_ratio() {
        assert_eq!(LodLevel::High.simplification_ratio(), 0.0);
        assert_eq!(LodLevel::Medium.simplification_ratio(), 0.5);
        assert_eq!(LodLevel::Low.simplification_ratio(), 0.9);
    }

    #[test]
    fn test_lod_level_face_multiplier() {
        assert_eq!(LodLevel::High.face_multiplier(), 1.0);
        assert_eq!(LodLevel::Medium.face_multiplier(), 0.5);
        assert_eq!(LodLevel::Low.face_multiplier(), 0.1);
    }

    #[test]
    fn test_lod_config_default() {
        let config = LodConfig::default();
        assert!(config.high_distance < config.medium_distance);
        assert!(config.medium_distance < config.low_distance);
    }

    #[test]
    #[should_panic(expected = "LOD distances must be positive and in ascending order")]
    fn test_lod_config_invalid_order() {
        LodConfig::new(100.0, 50.0, 10.0);
    }

    #[test]
    fn test_lod_manager_default() {
        let manager = LodManager::new();
        let config = manager.config();
        assert_eq!(config.high_distance, 10.0);
        assert_eq!(config.medium_distance, 50.0);
        assert_eq!(config.low_distance, 100.0);
    }

    #[test]
    fn test_lod_manager_get_level() {
        let manager = LodManager::new();

        // Close distance -> High LOD
        assert_eq!(manager.get_lod_level(5.0), LodLevel::High);

        // Medium distance -> Medium LOD
        assert_eq!(manager.get_lod_level(75.0), LodLevel::Medium);

        // Far distance -> Low LOD
        assert_eq!(manager.get_lod_level(150.0), LodLevel::Low);
    }

    #[test]
    fn test_lod_manager_custom_distances() {
        let mut manager = LodManager::new();
        manager.set_lod_distances(5.0, 25.0, 75.0);

        assert_eq!(manager.get_lod_level(3.0), LodLevel::High);
        assert_eq!(manager.get_lod_level(15.0), LodLevel::High);
        assert_eq!(manager.get_lod_level(50.0), LodLevel::Medium);
        assert_eq!(manager.get_lod_level(100.0), LodLevel::Low);
    }

    #[test]
    fn test_lod_manager_hysteresis() {
        let mut manager = LodManager::new();
        manager.set_hysteresis(0.2); // 20% hysteresis

        // With hysteresis, the boundaries are wider
        // At exactly the threshold, should still be High due to hysteresis
        assert_eq!(manager.get_lod_level(10.0), LodLevel::High);
    }

    #[test]
    fn test_lod_manager_interpolated_factor() {
        let manager = LodManager::new();

        // At zero distance -> 0.0 (High)
        assert_eq!(manager.get_interpolated_factor(0.0), 0.0);

        // At high distance boundary -> 0.0 (High)
        assert_eq!(manager.get_interpolated_factor(10.0), 0.0);

        // At medium distance boundary -> 0.5 (Medium)
        assert_eq!(manager.get_interpolated_factor(50.0), 0.5);

        // At low distance boundary -> 1.0 (Low)
        assert_eq!(manager.get_interpolated_factor(100.0), 1.0);

        // Beyond low distance -> 1.0 (Low)
        assert_eq!(manager.get_interpolated_factor(200.0), 1.0);
    }

    #[test]
    fn test_lod_manager_smooth_transitions() {
        let mut manager = LodManager::new();
        assert!(!manager.has_smooth_transitions());

        manager.set_smooth_transitions(true);
        assert!(manager.has_smooth_transitions());
    }

    #[test]
    #[should_panic(expected = "Hysteresis factor must be between 0.0 and 1.0")]
    fn test_lod_manager_invalid_hysteresis() {
        let mut manager = LodManager::new();
        manager.set_hysteresis(1.5);
    }

    #[test]
    fn test_lod_manager_with_state() {
        let manager = LodManager::new();

        // Previous: High, Distance: just beyond threshold with hysteresis
        // high_distance = 10.0, with default hysteresis 0.1, downgrade at 10.0 * 1.1 = 11.0
        let lod = manager.get_lod_level_with_state(12.0, LodLevel::High);
        assert_eq!(lod, LodLevel::Medium);

        // Previous: Low, Distance: just inside threshold
        // low_distance = 100.0, with hysteresis 0.1, upgrade at 100.0 * 0.9 = 90.0
        let lod = manager.get_lod_level_with_state(85.0, LodLevel::Low);
        assert_eq!(lod, LodLevel::Medium);
    }
}
